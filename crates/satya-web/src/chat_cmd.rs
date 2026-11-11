//! Slash-command parser and `/roll` (`/r`) pipe (`|>`) mini-DSL evaluator for the Session Chat input box.
//!
//! Supported commands:
//! - `/msg` (aliases: `/w`, `/whisper`) `@recipient1 @recipient2 {message}` — sends a targeted private message to the specified session participants.
//! - `/msg @broadcast {message}` — sends a public broadcast message to all participants in the session.
//! - `/roll <expr>` (alias: `/r <expr>`) — evaluates a dice expression and optional `|>` pipeline:
//!   - `/roll 2d10` or `/r d20` (defaults to `<dice>|>sum`, with bare `d<sides>` defaulting to `1d<sides>`)
//!   - `/roll 2d10|>sum`
//!   - `/roll 4d10|>hi(3)` or `/roll 5d20|>3` (take `n` highest)
//!   - `/roll 6d6|>lo(3)` (take `n` lowest)
//!   - `/roll 4d10|>gte(8)` (filter rolls `>= 8`, or check scalar `>= 8` after `|>sum`)
//!   - `/roll 5d6|>lte(3)` (filter rolls `<= 3`, or check scalar `<= 3` after `|>sum`)
//!   - `/roll 3d6|>sum|>gte(11)` and `/roll 3d6|>sum|>lte(11)` (scalar pass/fail threshold check)
//!   - `/roll 4d10e|>gte(8)` (exploding dice on maximum face value)
//!   - `/roll 4d10e(9)` (exploding dice on value `>= 9`)

use satya_schema::Participant;

/// Maximum number of bonus explosion rolls permitted in a single `/roll` invocation.
pub const MAX_EXPLOSIONS: usize = 100;

/// Parsed slash command entered into the Session Chat input box (`#chat-input`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatCommand {
    /// Broadcast message to all session participants (`/msg @broadcast {message}`).
    MsgBroadcast {
        /// Message text body to broadcast publicly.
        body: String,
    },
    /// Private targeted message to one or more `@recipient` handles (`/msg @recipient1 @recipient2 {message}`).
    MsgPrivate {
        /// Recipient handles stripped of their leading `@` character.
        targets: Vec<String>,
        /// Message text body to send privately.
        body: String,
    },
    /// Dice roll command with an optional `|>` transformation pipeline (`/roll <expr>`).
    Roll {
        /// Original raw expression string following `/roll`.
        expr_raw: String,
        /// Parsed dice generator and `|>` pipeline stages.
        pipeline: RollPipeline,
    },
    /// Command documentation and usage output (`/help`, `--help`, `/msg --help`, or `/roll --help`).
    Help {
        /// Optional topic (`"msg"` or `"roll"`) requested.
        topic: Option<String>,
        /// Formatted help text to display in the Session Chat log.
        help_text: String,
    },
}

/// Dice generator specification at the head of a `/roll` pipeline (`<count>d<sides>[e[(<threshold>)]]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiceSpec {
    /// Number of dice to roll initially (`1..=100`).
    pub count: usize,
    /// Number of faces on each die (`2..=1000`).
    pub sides: i64,
    /// Optional explosion threshold: `Some(sides)` for `e`, or `Some(n)` for `e(n)`.
    pub explode_at: Option<i64>,
}

/// A single transformation or reduction stage in a `/roll` pipe (`|>`) chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipeStage {
    /// Keep the `n` highest values from a list (`hi(n)` or shorthand `n`).
    Hi(usize),
    /// Keep the `n` lowest values from a list (`lo(n)`).
    Lo(usize),
    /// Filter list values `>= threshold`, or test whether a scalar value is `>= threshold` (`gte(n)`).
    Gte(i64),
    /// Filter list values `<= threshold`, or test whether a scalar value is `<= threshold` (`lte(n)`).
    Lte(i64),
    /// Sum all values in the list into a scalar (`sum`).
    Sum,
}

/// Complete parsed `/roll` pipeline consisting of a [`DiceSpec`] generator and ordered [`PipeStage`]s.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollPipeline {
    /// Initial dice generator (`<count>d<sides>[e[(<threshold>)]]`).
    pub dice: DiceSpec,
    /// Ordered pipeline stages separated by `|>` (defaults to `[PipeStage::Sum]` if none were specified).
    pub stages: Vec<PipeStage>,
    /// Whether the `Sum` stage was implicitly added as the default operation.
    pub implicit_sum: bool,
}

/// Value flowing between stages of a [`RollPipeline`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RollValue {
    /// A list of die or filtered integer values.
    List(Vec<i64>),
    /// A single reduced integer value (such as the output of `sum`).
    Scalar(i64),
    /// A pass/fail threshold comparison produced by piping a [`RollValue::Scalar`] into `gte(n)` or `lte(n)`.
    Check {
        /// The scalar value tested.
        value: i64,
        /// Comparison operator (`">="` or `"<="`).
        comparator: &'static str,
        /// Threshold target compared against `value`.
        target: i64,
        /// Whether the comparison succeeded (`true` = passed, `false` = failed).
        passed: bool,
    },
}

/// Individual die roll record captured during the initial dice generation step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RolledDie {
    /// Face value rolled (`1..=sides`).
    pub value: i64,
    /// Whether this die roll triggered an explosion (`value >= explode_at`).
    pub exploded: bool,
    /// Whether this die was generated as a bonus explosion roll.
    pub is_bonus_explosion: bool,
}

/// Complete result of evaluating a [`RollPipeline`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollOutcome {
    /// All dice rolled (including any bonus explosion rolls) in roll order.
    pub initial_dice: Vec<RolledDie>,
    /// Final value produced at the end of the `|>` pipeline.
    pub final_value: RollValue,
    /// Human-readable formatted summary suitable for broadcasting in Session Chat.
    pub summary: String,
}

/// Parses a chat input string starting with `/` (or `--help`) into a [`ChatCommand`].
///
/// Returns `Ok(None)` if `input` is a regular chat message.
/// Returns `Err(String)` with a user-facing syntax hint if `input` starts with `/` but is malformed.
pub fn parse_chat_command(input: &str) -> Result<Option<ChatCommand>, String> {
    let trimmed = input.trim();
    if trimmed.eq_ignore_ascii_case("--help") || trimmed.eq_ignore_ascii_case("-h") {
        return Ok(Some(build_help_command(None)));
    }
    if !trimmed.starts_with('/') {
        return Ok(None);
    }

    let (cmd, rest) = match trimmed.split_once(char::is_whitespace) {
        Some((c, r)) => (c, r.trim()),
        None => (trimmed, ""),
    };

    match cmd.to_ascii_lowercase().as_str() {
        "/help" | "/?" | "/--help" => {
            let topic = if rest.is_empty() {
                None
            } else {
                Some(rest.trim_start_matches('/').to_ascii_lowercase())
            };
            Ok(Some(build_help_command(topic.as_deref())))
        }
        "/msg" | "/w" | "/whisper" => {
            if is_help_flag(rest) {
                return Ok(Some(build_help_command(Some("msg"))));
            }
            parse_msg_command(rest).map(Some)
        }
        "/roll" | "/r" => {
            if is_help_flag(rest) {
                return Ok(Some(build_help_command(Some("roll"))));
            }
            if rest.is_empty() {
                return Err(
                    "Usage: /roll <dice>[|>stage...] (alias: /r; try '/roll --help' for examples)"
                        .to_string(),
                );
            }
            let pipeline = parse_roll_pipeline(rest)?;
            Ok(Some(ChatCommand::Roll {
                expr_raw: rest.to_string(),
                pipeline,
            }))
        }
        other => Err(format!(
            "Unknown command '{other}'. Type '/help' or '--help' to see supported commands (/msg, /w, /whisper, /roll, /r)."
        )),
    }
}

fn is_help_flag(rest: &str) -> bool {
    matches!(
        rest.trim().to_ascii_lowercase().as_str(),
        "--help" | "-h" | "help" | "?"
    )
}

/// Builds a [`ChatCommand::Help`] response for the requested topic (`None`, `Some("msg")` / `"w"` / `"whisper"`, or `Some("roll")` / `"r"`).
pub fn build_help_command(topic: Option<&str>) -> ChatCommand {
    let msg_help = "/msg (aliases: /w, /whisper) @recipient1 [@recipient2 ...] <message> — Send a private message to specific session participants.\n\
                    /msg @broadcast <message> — Broadcast a public message to all participants.";
    let roll_help = "/roll (alias: /r) <count>d<sides>[e[(<thresh>)]][|>stage...] — Roll dice through a pipe (|>) mini-DSL:\n\
                     • /roll 2d10, /r d20, or /roll 2d10|>sum (sum dice; default when no pipe is given)\n\
                     • /roll 4d10|>hi(3) or /roll 5d20|>3 (keep 3 highest rolls)\n\
                     • /roll 6d6|>lo(3) (keep 3 lowest rolls)\n\
                     • /roll 4d10|>gte(8) (keep rolls >= 8) | /roll 5d6|>lte(3) (keep rolls <= 3)\n\
                     • /roll 3d6|>sum|>gte(11) or |>lte(11) (pass/fail check on scalar sum)\n\
                     • /roll 4d10e|>gte(8) (explode on max face) | /roll 4d10e(9) (explode on >= 9)";

    match topic {
        Some("msg" | "w" | "whisper") => ChatCommand::Help {
            topic: Some("msg".to_string()),
            help_text: format!("[HELP: /msg (/w, /whisper)]\n{msg_help}"),
        },
        Some("roll" | "r") => ChatCommand::Help {
            topic: Some("roll".to_string()),
            help_text: format!("[HELP: /roll (/r)]\n{roll_help}"),
        },
        _ => ChatCommand::Help {
            topic: None,
            help_text: format!(
                "[HELP: Session Chat Commands]\n{msg_help}\n{roll_help}\nTip: Use Up/Down arrows in the chat box to cycle command history."
            ),
        },
    }
}

fn parse_msg_command(rest: &str) -> Result<ChatCommand, String> {
    if rest.is_empty() {
        return Err(
            "Usage: /msg @recipient1 [@recipient2 ...] <message> or /msg @broadcast <message>"
                .to_string(),
        );
    }

    let mut targets = Vec::new();
    let mut remainder = rest.trim_start();

    while remainder.starts_with('@') {
        let (token, after) = match remainder.split_once(char::is_whitespace) {
            Some((t, a)) => (t, a.trim_start()),
            None => (remainder, ""),
        };
        let handle = token.trim_start_matches('@').trim();
        if handle.is_empty() {
            return Err("Recipient handle after '@' cannot be empty.".to_string());
        }
        targets.push(handle.to_string());
        remainder = after;
    }

    if targets.is_empty() {
        return Err(
            "Usage: /msg requires at least one @recipient or @broadcast before the message."
                .to_string(),
        );
    }

    if remainder.is_empty() {
        return Err("Usage: /msg requires a message body after the @recipient(s).".to_string());
    }

    if targets.len() == 1
        && matches!(
            targets[0].to_ascii_lowercase().as_str(),
            "broadcast" | "all" | "public"
        )
    {
        return Ok(ChatCommand::MsgBroadcast {
            body: remainder.to_string(),
        });
    }

    // If @broadcast was mixed with other targets, treat @broadcast as broadcasting to all
    if targets.iter().any(|t| {
        matches!(
            t.to_ascii_lowercase().as_str(),
            "broadcast" | "all" | "public"
        )
    }) {
        return Ok(ChatCommand::MsgBroadcast {
            body: remainder.to_string(),
        });
    }

    Ok(ChatCommand::MsgPrivate {
        targets,
        body: remainder.to_string(),
    })
}

/// Resolves `@recipient` handles from a `/msg` command against the active session `roster`.
///
/// Returns `(matched_user_ids, unresolved_handles)`.
pub fn resolve_msg_recipients(
    targets: &[String],
    roster: &[Participant],
    self_user_id: Option<&str>,
) -> (Vec<String>, Vec<String>) {
    let mut matched_ids = Vec::new();
    let mut unresolved = Vec::new();

    for target in targets {
        let needle = target.trim().trim_start_matches('@').to_ascii_lowercase();
        if needle.is_empty() {
            continue;
        }

        let mut found_any = false;
        for p in roster {
            if self_user_id == Some(p.user_id.as_str()) {
                continue;
            }
            let uid_lower = p.user_id.to_ascii_lowercase();
            let uname_lower = p.username.to_ascii_lowercase();
            let disp_lower = p.display_name.to_ascii_lowercase();
            let disp_snake = disp_lower.replace(' ', "_");
            let disp_compact = disp_lower.replace(' ', "");
            let first_word = disp_lower
                .split(|c: char| c.is_whitespace() || c == '(' || c == '_')
                .next()
                .unwrap_or("");

            if uname_lower == needle
                || uid_lower == needle
                || (needle.len() >= 4 && uid_lower.starts_with(&needle))
                || disp_lower == needle
                || disp_snake == needle
                || disp_compact == needle
                || (!first_word.is_empty() && first_word == needle)
                || disp_lower.starts_with(&needle)
            {
                if !matched_ids.contains(&p.user_id) {
                    matched_ids.push(p.user_id.clone());
                }
                found_any = true;
            }
        }

        if !found_any {
            unresolved.push(target.clone());
        }
    }

    (matched_ids, unresolved)
}

/// Parses a `/roll` expression (everything after `/roll `) into a [`RollPipeline`].
pub fn parse_roll_pipeline(expr: &str) -> Result<RollPipeline, String> {
    let parts: Vec<&str> = expr.split("|>").map(str::trim).collect();
    if parts.is_empty() || parts[0].is_empty() {
        return Err(
            "Missing dice specification in /roll command (e.g. 2d10 or 4d10|>hi(3)).".to_string(),
        );
    }

    let dice = parse_dice_spec(parts[0])?;
    let mut stages = Vec::new();
    for raw_stage in &parts[1..] {
        if raw_stage.is_empty() {
            return Err("Empty pipe stage after '|>' in /roll command.".to_string());
        }
        stages.push(parse_pipe_stage(raw_stage)?);
    }

    let implicit_sum = stages.is_empty();
    if implicit_sum {
        stages.push(PipeStage::Sum);
    }

    Ok(RollPipeline {
        dice,
        stages,
        implicit_sum,
    })
}

fn parse_dice_spec(raw: &str) -> Result<DiceSpec, String> {
    let s = raw.trim().to_ascii_lowercase();
    let Some((count_str, after_d)) = s.split_once('d') else {
        return Err(format!(
            "Invalid dice notation '{raw}'. Expected <count>d<sides>[e[(<threshold>)]] (e.g. 4d10, 4d10e, 4d10e(9))."
        ));
    };

    let count: usize = if count_str.is_empty() {
        1
    } else {
        count_str.parse::<usize>().map_err(|_| {
            format!(
                "Invalid dice count '{count_str}' in '{raw}'. Must be an integer between 1 and 100."
            )
        })?
    };

    if !(1..=100).contains(&count) {
        return Err(format!(
            "Dice count {count} in '{raw}' must be between 1 and 100."
        ));
    }

    let (sides_str, explode_part) = match after_d.split_once('e') {
        Some((sides_s, exp_s)) => (sides_s, Some(exp_s)),
        None => (after_d, None),
    };

    let sides: i64 = sides_str.parse::<i64>().map_err(|_| {
        format!("Invalid die sides '{sides_str}' in '{raw}'. Must be an integer >= 2.")
    })?;

    if !(2..=1000).contains(&sides) {
        return Err(format!(
            "Die sides {sides} in '{raw}' must be between 2 and 1000."
        ));
    }

    let explode_at = match explode_part {
        None => None,
        Some("") => Some(sides),
        Some(exp_arg) => {
            if let Some(inner) = exp_arg.strip_prefix('(').and_then(|v| v.strip_suffix(')')) {
                let thresh: i64 = inner.trim().parse::<i64>().map_err(|_| {
                    format!("Invalid explosion threshold '{inner}' in '{raw}'. Expected integer inside e(...).")
                })?;
                if thresh < 2 || thresh > sides {
                    return Err(format!(
                        "Explosion threshold {thresh} in '{raw}' must be between 2 and {sides}."
                    ));
                }
                Some(thresh)
            } else {
                return Err(format!(
                    "Invalid explosion modifier 'e{exp_arg}' in '{raw}'. Use 'e' or 'e(<threshold>)'."
                ));
            }
        }
    };

    Ok(DiceSpec {
        count,
        sides,
        explode_at,
    })
}

fn parse_pipe_stage(raw: &str) -> Result<PipeStage, String> {
    let s = raw.trim().to_ascii_lowercase();

    // Bare integer shorthand: `|>3` is equivalent to `|>hi(3)`
    if let Ok(n) = s.parse::<usize>() {
        if n == 0 {
            return Err("Pipe stage count in '|>n' must be >= 1.".to_string());
        }
        return Ok(PipeStage::Hi(n));
    }

    if s == "sum" || s == "sum()" {
        return Ok(PipeStage::Sum);
    }

    if let Some(arg) = s.strip_prefix("hi(").and_then(|v| v.strip_suffix(')')) {
        let n = arg.trim().parse::<usize>().map_err(|_| {
            format!("Invalid argument '{arg}' in 'hi({arg})'. Expected positive integer.")
        })?;
        if n == 0 {
            return Err("Argument to hi(n) must be >= 1.".to_string());
        }
        return Ok(PipeStage::Hi(n));
    }

    if let Some(arg) = s.strip_prefix("lo(").and_then(|v| v.strip_suffix(')')) {
        let n = arg.trim().parse::<usize>().map_err(|_| {
            format!("Invalid argument '{arg}' in 'lo({arg})'. Expected positive integer.")
        })?;
        if n == 0 {
            return Err("Argument to lo(n) must be >= 1.".to_string());
        }
        return Ok(PipeStage::Lo(n));
    }

    if let Some(arg) = s.strip_prefix("gte(").and_then(|v| v.strip_suffix(')')) {
        let target = arg
            .trim()
            .parse::<i64>()
            .map_err(|_| format!("Invalid argument '{arg}' in 'gte({arg})'. Expected integer."))?;
        return Ok(PipeStage::Gte(target));
    }

    if let Some(arg) = s.strip_prefix("lte(").and_then(|v| v.strip_suffix(')')) {
        let target = arg
            .trim()
            .parse::<i64>()
            .map_err(|_| format!("Invalid argument '{arg}' in 'lte({arg})'. Expected integer."))?;
        return Ok(PipeStage::Lte(target));
    }

    Err(format!(
        "Unknown pipe stage '{raw}'. Supported stages: hi(n), lo(n), n, gte(n), lte(n), sum."
    ))
}

impl RollPipeline {
    /// Evaluates the dice roll and all `|>` pipeline stages using a caller-supplied die roller closure
    /// `roll_die(sides) -> 1..=sides`.
    pub fn evaluate_with_roller<F>(
        &self,
        expr_raw: &str,
        mut roll_die: F,
    ) -> Result<RollOutcome, String>
    where
        F: FnMut(i64) -> i64,
    {
        let mut initial_dice = Vec::new();
        let mut values = Vec::new();
        let mut explosions_remaining = MAX_EXPLOSIONS;

        for _ in 0..self.dice.count {
            let mut is_bonus = false;
            loop {
                let raw_val = roll_die(self.dice.sides).clamp(1, self.dice.sides);
                let explodes = self
                    .dice
                    .explode_at
                    .is_some_and(|thresh| raw_val >= thresh && explosions_remaining > 0);
                initial_dice.push(RolledDie {
                    value: raw_val,
                    exploded: explodes,
                    is_bonus_explosion: is_bonus,
                });
                values.push(raw_val);
                if explodes {
                    explosions_remaining -= 1;
                    is_bonus = true;
                } else {
                    break;
                }
            }
        }

        let mut current = RollValue::List(values);
        let mut trace_parts = Vec::new();

        for stage in &self.stages {
            match (&current, stage) {
                (RollValue::List(list), PipeStage::Hi(n)) => {
                    let mut sorted = list.clone();
                    sorted.sort_unstable_by(|a, b| b.cmp(a));
                    sorted.truncate(*n);
                    trace_parts.push(format!("hi({n}): [{}]", join_nums(&sorted)));
                    current = RollValue::List(sorted);
                }
                (RollValue::List(list), PipeStage::Lo(n)) => {
                    let mut sorted = list.clone();
                    sorted.sort_unstable();
                    sorted.truncate(*n);
                    trace_parts.push(format!("lo({n}): [{}]", join_nums(&sorted)));
                    current = RollValue::List(sorted);
                }
                (RollValue::List(list), PipeStage::Gte(target)) => {
                    let filtered: Vec<i64> =
                        list.iter().copied().filter(|v| *v >= *target).collect();
                    trace_parts.push(format!(
                        "gte({target}): [{}] ({} matching)",
                        join_nums(&filtered),
                        filtered.len()
                    ));
                    current = RollValue::List(filtered);
                }
                (RollValue::List(list), PipeStage::Lte(target)) => {
                    let filtered: Vec<i64> =
                        list.iter().copied().filter(|v| *v <= *target).collect();
                    trace_parts.push(format!(
                        "lte({target}): [{}] ({} matching)",
                        join_nums(&filtered),
                        filtered.len()
                    ));
                    current = RollValue::List(filtered);
                }
                (RollValue::List(list), PipeStage::Sum) => {
                    let total: i64 = list.iter().sum();
                    trace_parts.push(format!("Sum: {total}"));
                    current = RollValue::Scalar(total);
                }
                (RollValue::Scalar(val), PipeStage::Sum) => {
                    trace_parts.push(format!("Sum: {val}"));
                }
                (RollValue::Scalar(val), PipeStage::Gte(target)) => {
                    let passed = *val >= *target;
                    let status = if passed { "PASSED" } else { "FAILED" };
                    trace_parts.push(format!("gte({target}): {status} ({val} >= {target})"));
                    current = RollValue::Check {
                        value: *val,
                        comparator: ">=",
                        target: *target,
                        passed,
                    };
                }
                (RollValue::Scalar(val), PipeStage::Lte(target)) => {
                    let passed = *val <= *target;
                    let status = if passed { "PASSED" } else { "FAILED" };
                    trace_parts.push(format!("lte({target}): {status} ({val} <= {target})"));
                    current = RollValue::Check {
                        value: *val,
                        comparator: "<=",
                        target: *target,
                        passed,
                    };
                }
                (RollValue::Scalar(_), PipeStage::Hi(n)) => {
                    return Err(format!(
                        "Cannot pipe a scalar value into 'hi({n})'. Place 'hi({n})' before 'sum'."
                    ));
                }
                (RollValue::Scalar(_), PipeStage::Lo(n)) => {
                    return Err(format!(
                        "Cannot pipe a scalar value into 'lo({n})'. Place 'lo({n})' before 'sum'."
                    ));
                }
                (RollValue::Check { .. }, _) => {
                    return Err(
                        "Cannot pipe additional stages after a scalar pass/fail check.".to_string(),
                    );
                }
            }
        }

        let rolled_tokens: Vec<String> = initial_dice
            .iter()
            .map(|d| {
                let prefix = if d.is_bonus_explosion { "+" } else { "" };
                let suffix = if d.exploded { "!" } else { "" };
                format!("{prefix}{}{suffix}", d.value)
            })
            .collect();

        let summary = format!(
            "[ROLL] /roll {} => Rolled [{}] => {}",
            expr_raw.trim(),
            rolled_tokens.join(", "),
            trace_parts.join(" => ")
        );

        Ok(RollOutcome {
            initial_dice,
            final_value: current,
            summary,
        })
    }
}

fn join_nums(nums: &[i64]) -> String {
    nums.iter()
        .map(i64::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Maximum number of submitted chat messages/commands retained in [`ChatInputHistory`].
pub const MAX_CHAT_HISTORY: usize = 100;

/// Up/Down arrow history buffer for the Session Chat input box (`#chat-input`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChatInputHistory {
    /// Chronological list of submitted non-empty messages/commands (oldest first, newest last).
    entries: Vec<String>,
    /// Current navigation index into `entries` (`None` when editing the live bottom draft).
    cursor: Option<usize>,
    /// Unsent input draft saved when the user begins pressing `ArrowUp`.
    draft: String,
}

impl ChatInputHistory {
    /// Creates an empty [`ChatInputHistory`] buffer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a newly submitted chat message or slash command and resets the navigation cursor.
    pub fn record_sent(&mut self, text: &str) {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            self.cursor = None;
            self.draft.clear();
            return;
        }
        if self.entries.last().map(String::as_str) != Some(trimmed) {
            if self.entries.len() >= MAX_CHAT_HISTORY {
                self.entries.remove(0);
            }
            self.entries.push(trimmed.to_string());
        }
        self.cursor = None;
        self.draft.clear();
    }

    /// Moves backward (`ArrowUp`) through submitted chat history, saving `current_input` as the draft
    /// when starting navigation from the bottom.
    pub fn navigate_up(&mut self, current_input: &str) -> Option<String> {
        if self.entries.is_empty() {
            return None;
        }
        let next_idx = match self.cursor {
            None => {
                self.draft = current_input.to_string();
                self.entries.len() - 1
            }
            Some(0) => 0,
            Some(idx) => idx - 1,
        };
        self.cursor = Some(next_idx);
        self.entries.get(next_idx).cloned()
    }

    /// Moves forward (`ArrowDown`) through submitted chat history, restoring the saved draft
    /// when advancing past the newest entry.
    pub fn navigate_down(&mut self) -> Option<String> {
        let idx = self.cursor?;
        if idx + 1 < self.entries.len() {
            let next_idx = idx + 1;
            self.cursor = Some(next_idx);
            self.entries.get(next_idx).cloned()
        } else {
            self.cursor = None;
            Some(self.draft.clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use satya_schema::SessionRole;

    fn make_roster() -> Vec<Participant> {
        vec![
            Participant::human(
                "11111111-aaaa-bbbb-cccc-000000000001",
                "gm_sean",
                "Sean (GM)",
                SessionRole::Gm,
            ),
            Participant::human(
                "22222222-aaaa-bbbb-cccc-000000000002",
                "sara_valkyrie",
                "Sara (Player 1)",
                SessionRole::Player,
            ),
            Participant::human(
                "33333333-aaaa-bbbb-cccc-000000000003",
                "marcus_pilot",
                "Marcus (Player 2)",
                SessionRole::Player,
            ),
        ]
    }

    #[test]
    fn test_parse_msg_private_and_broadcast_commands() {
        let cmd1 = parse_chat_command("/msg @sara @marcus Hold your fire!")
            .unwrap()
            .unwrap();
        assert_eq!(
            cmd1,
            ChatCommand::MsgPrivate {
                targets: vec!["sara".to_string(), "marcus".to_string()],
                body: "Hold your fire!".to_string(),
            }
        );

        let roster = make_roster();
        if let ChatCommand::MsgPrivate { targets, .. } = &cmd1 {
            let (matched, unresolved) = resolve_msg_recipients(
                targets,
                &roster,
                Some("11111111-aaaa-bbbb-cccc-000000000001"),
            );
            assert_eq!(
                matched,
                vec![
                    "22222222-aaaa-bbbb-cccc-000000000002".to_string(),
                    "33333333-aaaa-bbbb-cccc-000000000003".to_string()
                ]
            );
            assert!(unresolved.is_empty());
        }

        let cmd2 = parse_chat_command("/msg @broadcast Everyone regroup at airlock 3")
            .unwrap()
            .unwrap();
        assert_eq!(
            cmd2,
            ChatCommand::MsgBroadcast {
                body: "Everyone regroup at airlock 3".to_string(),
            }
        );

        assert_eq!(parse_chat_command("normal chat message").unwrap(), None);
    }

    fn eval_seq(expr: &str, seq: &[i64]) -> (RollPipeline, RollOutcome) {
        let p = parse_roll_pipeline(expr).unwrap();
        let mut idx = 0;
        let out = p
            .evaluate_with_roller(expr, |_| {
                let v = seq[idx];
                idx += 1;
                v
            })
            .unwrap();
        (p, out)
    }

    #[test]
    fn test_roll_dsl_hi_lo_and_shorthand() {
        let (_, out_hi) = eval_seq("4d10|>hi(3)", &[3, 9, 6, 8]);
        assert_eq!(out_hi.final_value, RollValue::List(vec![9, 8, 6]));
        assert!(out_hi.summary.contains("hi(3): [9, 8, 6]"));

        let (_, out_lo) = eval_seq("6d6|>lo(3)", &[5, 2, 6, 1, 4, 3]);
        assert_eq!(out_lo.final_value, RollValue::List(vec![1, 2, 3]));
        assert!(out_lo.summary.contains("lo(3): [1, 2, 3]"));

        let (p_short, out_short) = eval_seq("5d20|>3", &[4, 19, 12, 7, 15]);
        assert_eq!(p_short.stages, vec![PipeStage::Hi(3)]);
        assert_eq!(out_short.final_value, RollValue::List(vec![19, 15, 12]));
    }

    #[test]
    fn test_roll_dsl_gte_lte_and_sum_pipelines() {
        let (_, out_gte) = eval_seq("4d10|>gte(8)", &[8, 3, 10, 7]);
        assert_eq!(out_gte.final_value, RollValue::List(vec![8, 10]));

        let (_, out_lte) = eval_seq("5d6|>lte(3)", &[4, 2, 6, 3, 1]);
        assert_eq!(out_lte.final_value, RollValue::List(vec![2, 3, 1]));

        let (p_default, out_default) = eval_seq("2d10", &[7, 6]);
        assert!(p_default.implicit_sum);
        assert_eq!(p_default.stages, vec![PipeStage::Sum]);
        assert_eq!(out_default.final_value, RollValue::Scalar(13));
        assert!(out_default.summary.contains("Sum: 13"));

        let (_, out_sum_gte) = eval_seq("3d6|>sum|>gte(11)", &[4, 5, 3]);
        assert_eq!(
            out_sum_gte.final_value,
            RollValue::Check {
                value: 12,
                comparator: ">=",
                target: 11,
                passed: true,
            }
        );
        assert!(out_sum_gte.summary.contains("PASSED (12 >= 11)"));

        let (_, out_sum_lte) = eval_seq("3d6|>sum|>lte(11)", &[4, 5, 3]);
        assert_eq!(
            out_sum_lte.final_value,
            RollValue::Check {
                value: 12,
                comparator: "<=",
                target: 11,
                passed: false,
            }
        );
        assert!(out_sum_lte.summary.contains("FAILED (12 <= 11)"));
    }

    #[test]
    fn test_roll_dsl_exploding_dice() {
        let (p_exp, out_exp) = eval_seq("4d10e|>gte(8)", &[10, 8, 4, 9, 2]);
        assert_eq!(p_exp.dice.explode_at, Some(10));
        assert_eq!(out_exp.initial_dice.len(), 5);
        assert!(out_exp.initial_dice[0].exploded);
        assert!(out_exp.initial_dice[1].is_bonus_explosion);
        assert_eq!(out_exp.final_value, RollValue::List(vec![10, 8, 9]));
        assert!(out_exp.summary.contains("10!, +8"));

        let (p_exp9, out_exp9) = eval_seq("4d10e(9)", &[9, 5, 3, 10, 4, 2]);
        assert_eq!(p_exp9.dice.explode_at, Some(9));
        assert_eq!(p_exp9.stages, vec![PipeStage::Sum]);
        assert_eq!(out_exp9.initial_dice.len(), 6);
        assert_eq!(out_exp9.final_value, RollValue::Scalar(33));
    }

    #[test]
    fn test_help_flags_and_chat_input_history() {
        for flag in ["--help", "/help", "/msg --help", "/roll --help", "/roll -h"] {
            let parsed = parse_chat_command(flag).unwrap();
            assert!(
                matches!(parsed, Some(ChatCommand::Help { .. })),
                "expected Help variant for '{flag}'"
            );
        }

        if let Some(ChatCommand::Help { topic, help_text }) =
            parse_chat_command("/roll --help").unwrap()
        {
            assert_eq!(topic.as_deref(), Some("roll"));
            assert!(help_text.contains("4d10|>hi(3)"));
            assert!(help_text.contains("4d10e(9)"));
        }

        let mut hist = ChatInputHistory::new();
        assert_eq!(hist.navigate_up("draft"), None);

        hist.record_sent("/roll 2d10");
        hist.record_sent("/roll 4d10|>hi(3)");
        // Consecutive duplicate should be deduplicated
        hist.record_sent("/roll 4d10|>hi(3)");
        hist.record_sent("/msg @sara Ready?");

        assert_eq!(
            hist.navigate_up("in-progress text"),
            Some("/msg @sara Ready?".to_string())
        );
        assert_eq!(
            hist.navigate_up("ignored while browsing"),
            Some("/roll 4d10|>hi(3)".to_string())
        );
        assert_eq!(hist.navigate_up(""), Some("/roll 2d10".to_string()));
        // Clamps at oldest entry
        assert_eq!(hist.navigate_up(""), Some("/roll 2d10".to_string()));

        // Navigate back down to draft
        assert_eq!(hist.navigate_down(), Some("/roll 4d10|>hi(3)".to_string()));
        assert_eq!(hist.navigate_down(), Some("/msg @sara Ready?".to_string()));
        assert_eq!(hist.navigate_down(), Some("in-progress text".to_string()));
        assert_eq!(hist.navigate_down(), None);
    }

    #[test]
    fn test_command_aliases_r_w_whisper_and_help_topics() {
        let r_cmd = parse_chat_command("/r d20").unwrap().unwrap();
        assert_eq!(
            r_cmd,
            ChatCommand::Roll {
                expr_raw: "d20".to_string(),
                pipeline: RollPipeline {
                    dice: DiceSpec {
                        count: 1,
                        sides: 20,
                        explode_at: None,
                    },
                    stages: vec![PipeStage::Sum],
                    implicit_sum: true,
                },
            }
        );

        let w_cmd = parse_chat_command("/w @sara Eyes on the bulkhead")
            .unwrap()
            .unwrap();
        assert_eq!(
            w_cmd,
            ChatCommand::MsgPrivate {
                targets: vec!["sara".to_string()],
                body: "Eyes on the bulkhead".to_string(),
            }
        );

        let whisper_cmd = parse_chat_command("/whisper @marcus @sara Cut comms")
            .unwrap()
            .unwrap();
        assert_eq!(
            whisper_cmd,
            ChatCommand::MsgPrivate {
                targets: vec!["marcus".to_string(), "sara".to_string()],
                body: "Cut comms".to_string(),
            }
        );

        for (raw, expected_topic) in [
            ("/r --help", Some("roll")),
            ("/help r", Some("roll")),
            ("/w --help", Some("msg")),
            ("/whisper -h", Some("msg")),
            ("/help w", Some("msg")),
            ("/help whisper", Some("msg")),
        ] {
            match parse_chat_command(raw).unwrap() {
                Some(ChatCommand::Help { topic, help_text }) => {
                    assert_eq!(topic.as_deref(), expected_topic, "mismatch for '{raw}'");
                    assert!(!help_text.is_empty());
                }
                other => panic!("expected Help for '{raw}', got {other:?}"),
            }
        }
    }

    #[test]
    fn test_explosion_cap_history_eviction_self_exclusion_and_malformed_errors() {
        // MAX_EXPLOSIONS cap terminates cleanly even when die always rolls max face
        let p_inf = parse_roll_pipeline("1d6e").unwrap();
        let out_inf = p_inf.evaluate_with_roller("1d6e", |_| 6).unwrap();
        assert_eq!(out_inf.initial_dice.len(), 1 + MAX_EXPLOSIONS);
        assert!(out_inf.initial_dice[0].exploded);
        assert!(out_inf.initial_dice[MAX_EXPLOSIONS - 1].exploded);
        assert!(!out_inf.initial_dice[MAX_EXPLOSIONS].exploded);
        assert!(out_inf.initial_dice[MAX_EXPLOSIONS].is_bonus_explosion);
        assert_eq!(
            out_inf.final_value,
            RollValue::Scalar(6 * (1 + MAX_EXPLOSIONS as i64))
        );

        // MAX_CHAT_HISTORY ring buffer evicts oldest entries past 100
        let mut hist = ChatInputHistory::new();
        for idx in 0..(MAX_CHAT_HISTORY + 5) {
            hist.record_sent(&format!("entry-{idx}"));
        }
        let mut oldest = String::new();
        for _ in 0..(MAX_CHAT_HISTORY + 2) {
            oldest = hist.navigate_up("").unwrap();
        }
        assert_eq!(oldest, "entry-5");

        // Self exclusion + unresolved handles in resolve_msg_recipients
        let roster = make_roster();
        let targets = vec![
            "gm_sean".to_string(),
            "sara".to_string(),
            "ghost_operator".to_string(),
        ];
        let (matched, unresolved) = resolve_msg_recipients(
            &targets,
            &roster,
            Some("11111111-aaaa-bbbb-cccc-000000000001"),
        );
        assert_eq!(
            matched,
            vec!["22222222-aaaa-bbbb-cccc-000000000002".to_string()]
        );
        assert_eq!(
            unresolved,
            vec!["gm_sean".to_string(), "ghost_operator".to_string()]
        );

        // Malformed dice specs and pipelines return Err(...)
        for bad_expr in ["0d6", "101d6", "2d1", "4d6|>hi(0)", "4d6|>"] {
            assert!(
                parse_roll_pipeline(bad_expr).is_err(),
                "expected parse error for '{bad_expr}'"
            );
        }

        // Runtime type errors in pipeline evaluation: scalar |> hi(1), and check |> sum
        let scalar_into_hi = parse_roll_pipeline("4d6|>sum|>hi(1)").unwrap();
        assert!(
            scalar_into_hi
                .evaluate_with_roller("4d6|>sum|>hi(1)", |_| 4)
                .is_err()
        );

        let after_check = parse_roll_pipeline("3d6|>sum|>gte(10)|>sum").unwrap();
        assert!(
            after_check
                .evaluate_with_roller("3d6|>sum|>gte(10)|>sum", |_| 4)
                .is_err()
        );
    }
}
