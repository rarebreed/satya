"""Auto-generate and verify ``src/satya_py/_native.pyi`` from Rust PyO3 sources and runtime metadata.

Combines Rust AST/regex extraction across ``crates/satya-py/src/*.rs`` (for exact parameter types,
return types, ``#[pyclass]`` fields, ``#[getter]`` properties, and ``#[staticmethod]``/``#[classmethod]``
decorators) with compiled ``satya_py._native`` introspection (``inspect.signature``,
``cls.__text_signature__``, and ``dir(cls)``) so ``_native.pyi`` can never drift silently from Rust.
"""

from __future__ import annotations

import argparse
import difflib
import inspect
import re
import sys
from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path
from types import MappingProxyType
from typing import Any, Final

from satya_py._loader import ensure_native_loaded


@dataclass(frozen=True, slots=True)
class _RustSourceSpec:
    filename: str
    section_title: str


_RUST_SOURCES: Final[tuple[_RustSourceSpec, ...]] = (
    _RustSourceSpec("lib.rs", "Core dice probabilities, cascading task params, combat pools, rules book & Avro schema registry"),
    _RustSourceSpec("tactical_map_py.rs", "Continuous metric tactical maps, LoS/cover, reachability, ballistics & spatial edits"),
    _RustSourceSpec("hooks_and_npc_py.rs", "Motivational hook weaver, procedural NPCs, lifepath anchors & embedded campaign config"),
    _RustSourceSpec("psyche_social_py.rs", "9-axis Psyche, social combat, cognitive dissonance, morale & world ledgers"),
    _RustSourceSpec("awakened_systems_py.rs", "Awakened AI substrates, L0/L1 cages, conspiracy ledgers, polycrisis & tradecraft"),
    _RustSourceSpec("tech_and_gear_py.rs", "Ch. 7, 11, 18, and 20-24 stealth, contacts, ordnance, vehicles, hazards, drones & EW"),
)

_PYCLASS_HEADER_RE: Final[re.Pattern[str]] = re.compile(
    r"(?P<doc>(?:[ \t]*///[^\n]*\n)*)"
    r"(?:[ \t]*#\[(?!pyclass\b)[^\n]*\n)*"
    r"[ \t]*#\[pyclass\((?P<attrs>[^)]*)\)\]"
    r"(?:\s*#\[[^\n]*\])*\s*"
    r"pub struct (?P<rust_name>\w+)\s*\{",
    re.MULTILINE,
)

_PYMETHODS_HEADER_RE: Final[re.Pattern[str]] = re.compile(
    r"#\[pymethods\]\s*impl (?P<rust_name>\w+)\s*\{",
    re.MULTILINE,
)

_ATTR_OR_DOC: Final[str] = (
    r"(?:[ \t]*///[^\n]*\n|[ \t]*#\[(?:[^\[\]]|\[(?:[^\[\]]|\[[^\[\]]*\])*\])*\]\s*\n)"
)

_FN_DECL_RE: Final[re.Pattern[str]] = re.compile(
    rf"(?P<attrs>(?:{_ATTR_OR_DOC})*)[ \t]*(?:pub )?fn (?P<name>\w+)(?:<[^>]*>)?\s*\(",
    re.MULTILINE,
)

_RUST_SIMPLE_TYPES: Final[Mapping[str, str]] = MappingProxyType(
    {
        "&str": "str",
        "String": "str",
        "bool": "bool",
        "f32": "float",
        "f64": "float",
        "i8": "int",
        "i16": "int",
        "i32": "int",
        "i64": "int",
        "isize": "int",
        "u8": "int",
        "u16": "int",
        "u32": "int",
        "u64": "int",
        "usize": "int",
        "PyJsonDict": "dict[str, Any]",
        "PyJsonList": "list[dict[str, Any]]",
        "Py<PyDict>": "dict[str, Any]",
        "Bound<'_, PyDict>": "dict[str, Any]",
        "Bound<'py, PyDict>": "dict[str, Any]",
        "Py<PyList>": "list[Any]",
        "PyJsonInput<'_>": "Mapping[str, Any] | str",
        "PyListInput<'_>": "Sequence[Any] | str",
    }
)
_RUST_PMF_TYPES: Final[frozenset[str]] = frozenset({"BTreeMap<i32, f64>", "Pmf"})
_RUST_CLASS_MAP: dict[str, str] = {}


@dataclass(frozen=True, slots=True)
class _ParsedMethod:
    name: str
    params: dict[str, str]
    return_type: str
    is_getter: bool
    is_staticmethod: bool
    is_classmethod: bool


@dataclass(slots=True)
class _ParsedClass:
    py_name: str
    rust_name: str
    get_all: bool
    fields: dict[str, str]
    methods: dict[str, _ParsedMethod]


@dataclass(frozen=True, slots=True)
class _ParsedFunction:
    name: str
    params: dict[str, str]
    return_type: str
    source_file: str


def _repo_root() -> Path:
    return Path(__file__).resolve().parent.parent.parent


def _extract_balanced(text: str, start_idx: int, open_ch: str, close_ch: str) -> tuple[str, int]:
    depth = 1
    i = start_idx
    n = len(text)
    in_str = False
    escape = False
    while i < n and depth > 0:
        ch = text[i]
        if in_str:
            if escape:
                escape = False
            elif ch == "\\":
                escape = True
            elif ch == '"':
                in_str = False
        else:
            if ch == '"':
                in_str = True
            elif ch == open_ch:
                depth += 1
            elif ch == close_ch:
                depth -= 1
                if depth == 0:
                    return text[start_idx:i], i + 1
        i += 1
    raise ValueError(f"Unbalanced '{open_ch}{close_ch}' starting at offset {start_idx}")


def _split_top_level(text: str, sep: str = ",") -> list[str]:
    parts: list[str] = []
    buf: list[str] = []
    depth = 0
    for ch in text:
        if ch in "(<[":
            depth += 1
        elif ch in ")>]":
            depth = max(0, depth - 1)
        if ch == sep and depth == 0:
            item = "".join(buf).strip()
            if item:
                parts.append(item)
            buf = []
        else:
            buf.append(ch)
    tail = "".join(buf).strip()
    if tail:
        parts.append(tail)
    return parts


def _rust_type_to_py(raw_type: str, *, for_param: bool, self_class: str = "") -> str:
    t = " ".join(raw_type.split())
    t = re.sub(r"\s*<\s*", "<", t)
    t = re.sub(r"\s*>\s*", ">", t)
    t = re.sub(r"\s*,\s*", ", ", t)

    if t == "()" or not t:
        return "None"
    if t == "Self":
        return self_class or "Any"
    if t.startswith("PyResult<") and t.endswith(">"):
        return _rust_type_to_py(t[9:-1], for_param=for_param, self_class=self_class)
    if t.startswith("Option<") and t.endswith(">"):
        inner = _rust_type_to_py(t[7:-1], for_param=for_param, self_class=self_class)
        return inner if inner.endswith("| None") else f"{inner} | None"

    if t in _RUST_SIMPLE_TYPES:
        return _RUST_SIMPLE_TYPES[t]
    if t in _RUST_PMF_TYPES:
        return "Mapping[int, float]" if for_param else "dict[int, float]"
    if t in _RUST_CLASS_MAP:
        return _RUST_CLASS_MAP[t]
    if t.startswith("Py<") and t.endswith(">"):
        inner = t[3:-1].strip()
        if inner in _RUST_CLASS_MAP:
            return _RUST_CLASS_MAP[inner]

    if t.startswith("Vec<") and t.endswith(">"):
        inner = _rust_type_to_py(t[4:-1], for_param=for_param, self_class=self_class)
        container = "Sequence" if for_param else "list"
        return f"{container}[{inner}]"

    if t.startswith("(") and t.endswith(")"):
        elems = [_rust_type_to_py(p, for_param=for_param, self_class=self_class) for p in _split_top_level(t[1:-1])]
        return f"tuple[{', '.join(elems)}]"

    return "Any"


def _parse_params_block(params_raw: str) -> dict[str, str]:
    parsed: dict[str, str] = {}
    for item in _split_top_level(params_raw):
        cleaned = re.sub(r"^\s*mut\s+", "", item).strip()
        if not cleaned or cleaned in {"&self", "&mut self", "self", "mut self"} or cleaned.startswith("&"):
            continue
        if ":" not in cleaned:
            continue
        name, rtype = cleaned.split(":", 1)
        name = name.strip()
        rtype_clean = " ".join(rtype.split())
        if name in {"py", "_py"} or rtype_clean.startswith(("Python<", "&Bound<", "Bound<")):
            if rtype_clean.startswith(("Python<", "&Bound<")):
                continue
        parsed[name] = _rust_type_to_py(rtype_clean, for_param=True)
    return parsed


def _parse_struct_fields(body: str, get_all: bool) -> dict[str, str]:
    fields: dict[str, str] = {}
    has_get_attr = False
    for raw_line in body.splitlines():
        stripped = raw_line.strip()
        if not stripped or stripped.startswith("//"):
            continue
        if stripped.startswith("#[pyo3(") and "get" in stripped:
            has_get_attr = True
            continue
        if stripped.startswith("#["):
            continue
        if stripped.startswith("pub ") and ":" in stripped:
            if get_all or has_get_attr:
                decl = stripped[4:].rstrip(",")
                fname, ftype = decl.split(":", 1)
                fields[fname.strip()] = _rust_type_to_py(ftype.strip(), for_param=False)
        has_get_attr = False
    return fields


def _parse_pymethods_block(body: str, py_class_name: str) -> dict[str, _ParsedMethod]:
    methods: dict[str, _ParsedMethod] = {}
    pos = 0
    while True:
        m = _FN_DECL_RE.search(body, pos)
        if m is None:
            break
        attrs = m.group("attrs")
        fn_name = m.group("name")
        params_raw, after_params = _extract_balanced(body, m.end(), "(", ")")
        rest = body[after_params:]
        brace_rel = rest.find("{")
        if brace_rel < 0:
            break
        header_tail = rest[:brace_rel].strip()
        ret_raw = "()"
        if header_tail.startswith("->"):
            ret_raw = header_tail[2:].strip()
        _, after_body = _extract_balanced(body, after_params + brace_rel + 1, "{", "}")
        pos = after_body

        py_name = "new" if fn_name == "new" and "#[new]" in attrs else fn_name
        is_getter = "#[getter]" in attrs
        is_staticmethod = "#[staticmethod]" in attrs
        is_classmethod = "#[classmethod]" in attrs
        if py_name.startswith("_") and py_name != "__repr__":
            continue
        params = _parse_params_block(params_raw)
        if py_name == "new":
            ret_type = "None"
        else:
            ret_type = _rust_type_to_py(ret_raw, for_param=False, self_class=py_class_name)
        methods[py_name] = _ParsedMethod(
            name=py_name,
            params=params,
            return_type=ret_type,
            is_getter=is_getter,
            is_staticmethod=is_staticmethod,
            is_classmethod=is_classmethod,
        )
    return methods


def _parse_rust_sources(crates_dir: Path) -> tuple[dict[str, _ParsedClass], dict[str, _ParsedFunction]]:
    raw_texts = {
        spec.filename: (crates_dir / spec.filename).read_text(encoding="utf-8")
        for spec in _RUST_SOURCES
    }

    _RUST_CLASS_MAP.clear()
    class_headers: list[tuple[str, str, bool, str]] = []
    for fname, txt in raw_texts.items():
        for m in _PYCLASS_HEADER_RE.finditer(txt):
            attrs = m.group("attrs")
            rust_name = m.group("rust_name")
            name_m = re.search(r'name\s*=\s*"([^"]+)"', attrs)
            py_name = name_m.group(1) if name_m else rust_name
            get_all = "get_all" in attrs
            _RUST_CLASS_MAP[rust_name] = py_name
            class_headers.append((fname, rust_name, get_all, py_name))

    classes: dict[str, _ParsedClass] = {}
    for fname, rust_name, get_all, py_name in class_headers:
        txt = raw_texts[fname]
        sm = re.search(rf"pub struct {rust_name}\s*\{{", txt)
        assert sm is not None
        struct_body, _ = _extract_balanced(txt, sm.end(), "{", "}")
        fields = _parse_struct_fields(struct_body, get_all=get_all)
        classes[py_name] = _ParsedClass(
            py_name=py_name,
            rust_name=rust_name,
            get_all=get_all,
            fields=fields,
            methods={},
        )

    rust_to_py = {c.rust_name: c for c in classes.values()}
    for txt in raw_texts.values():
        for m in _PYMETHODS_HEADER_RE.finditer(txt):
            cls = rust_to_py.get(m.group("rust_name"))
            if cls is None:
                continue
            impl_body, _ = _extract_balanced(txt, m.end(), "{", "}")
            cls.methods.update(_parse_pymethods_block(impl_body, cls.py_name))

    functions: dict[str, _ParsedFunction] = {}
    for fname, txt in raw_texts.items():
        pos = 0
        while True:
            m = _FN_DECL_RE.search(txt, pos)
            if m is None:
                break
            attrs = m.group("attrs")
            fn_name = m.group("name")
            params_raw, after_params = _extract_balanced(txt, m.end(), "(", ")")
            pos = after_params
            if "#[pyfunction]" not in attrs:
                continue
            rest = txt[after_params:]
            brace_rel = rest.find("{")
            if brace_rel < 0:
                continue
            header_tail = rest[:brace_rel].strip()
            ret_raw = header_tail[2:].strip() if header_tail.startswith("->") else "()"
            params = _parse_params_block(params_raw)
            ret_type = _rust_type_to_py(ret_raw, for_param=False)
            functions[fn_name] = _ParsedFunction(
                name=fn_name,
                params=params,
                return_type=ret_type,
                source_file=fname,
            )

    return classes, functions


def _format_default(val: Any) -> str:
    if isinstance(val, str):
        escaped = val.replace("\\", "\\\\").replace('"', '\\"')
        return f'"{escaped}"'
    return repr(val)


def _format_param_list(
    sig: inspect.Signature,
    rust_param_types: dict[str, str],
    *,
    leading_self: str | None = None,
) -> list[str]:
    rendered: list[str] = []
    if leading_self is not None:
        rendered.append(leading_self)
    params = list(sig.parameters.values())
    for idx, p in enumerate(params):
        if idx == 0 and p.name in {"self", "cls"}:
            continue
        ann = rust_param_types.get(p.name, "Any")
        piece = f"{p.name}: {ann}"
        if p.default is not inspect.Parameter.empty:
            piece += f" = {_format_default(p.default)}"
        rendered.append(piece)
    return rendered


def _emit_def_lines(
    name: str,
    params: list[str],
    ret_type: str,
    *,
    indent: str = "",
    decorators: tuple[str, ...] = (),
) -> list[str]:
    lines = [f"{indent}{d}" for d in decorators]
    one_line = f"{indent}def {name}({', '.join(params)}) -> {ret_type}: ..."
    if len(one_line) <= 108 or len(params) <= 2:
        lines.append(one_line)
        return lines
    lines.append(f"{indent}def {name}(")
    for p in params:
        lines.append(f"{indent}    {p},")
    lines.append(f"{indent}) -> {ret_type}: ...")
    return lines


def generate_native_stub() -> str:
    """Generate the complete PEP 484 stub content for ``satya_py._native`` with LF endings."""
    native = ensure_native_loaded()
    crates_dir = _repo_root() / "crates" / "satya-py" / "src"
    parsed_classes, parsed_funcs = _parse_rust_sources(crates_dir)

    runtime_class_names = sorted(
        n for n in dir(native) if not n.startswith("_") and isinstance(getattr(native, n), type)
    )
    runtime_func_names = sorted(
        n for n in dir(native) if not n.startswith("_") and not isinstance(getattr(native, n), type)
    )

    missing_cls = [c for c in runtime_class_names if c not in parsed_classes]
    if missing_cls:
        raise RuntimeError(f"Rust parser missed runtime #[pyclass] types: {missing_cls}")
    missing_fns = [f for f in runtime_func_names if f not in parsed_funcs]
    if missing_fns:
        raise RuntimeError(f"Rust parser missed runtime #[pyfunction] exports: {missing_fns}")

    out: list[str] = [
        '"""Auto-generated PEP 484 type stubs for the compiled Rust ``satya_py._native`` PyO3 extension."""',
        "",
        "from __future__ import annotations",
        "",
        "from collections.abc import Mapping, Sequence",
        "from typing import Any",
        "",
    ]

    for cls_name in runtime_class_names:
        cls_obj = getattr(native, cls_name)
        cls_spec = parsed_classes[cls_name]
        out.append("")
        out.append(f"class {cls_name}:")
        body_lines: list[str] = []

        # Emit struct fields in declaration order (filtered to runtime dir unless hidden)
        runtime_members = {n for n in dir(cls_obj) if not n.startswith("_")}
        for fname, ftype in cls_spec.fields.items():
            if fname in runtime_members:
                body_lines.append(f"    {fname}: {ftype}")

        # Emit __init__ whenever PyO3 exposes __text_signature__ on the class
        if getattr(cls_obj, "__text_signature__", None) is not None:
            init_sig = inspect.signature(cls_obj)
            new_spec = cls_spec.methods.get("new")
            param_types = new_spec.params if new_spec else {}
            init_params = _format_param_list(init_sig, param_types, leading_self="self")
            body_lines.extend(_emit_def_lines("__init__", init_params, "None", indent="    "))

        # Emit #[getter] properties and #[pymethods] functions
        for mname, mspec in cls_spec.methods.items():
            if mname == "new":
                continue
            if mname == "__repr__":
                body_lines.append("    def __repr__(self) -> str: ...")
                continue
            if mname not in runtime_members:
                continue
            if mspec.is_getter:
                body_lines.append("    @property")
                body_lines.append(f"    def {mname}(self) -> {mspec.return_type}: ...")
                continue

            member_obj = getattr(cls_obj, mname)
            decorators: list[str] = []
            leading_self: str | None = "self"
            if mspec.is_staticmethod:
                decorators.append("@staticmethod")
                leading_self = None
            elif mspec.is_classmethod:
                decorators.append("@classmethod")
                leading_self = "cls"

            if getattr(member_obj, "__text_signature__", None) is not None:
                msig = inspect.signature(member_obj)
                mparams = _format_param_list(msig, mspec.params, leading_self=leading_self)
            else:
                mparams = [leading_self] if leading_self else []
                mparams.extend(f"{pn}: {pt}" for pn, pt in mspec.params.items())
            body_lines.extend(
                _emit_def_lines(
                    mname,
                    mparams,
                    mspec.return_type,
                    indent="    ",
                    decorators=tuple(decorators),
                )
            )

        out.extend(body_lines or ["    ..."])

    # Emit module functions grouped by source file
    for spec in _RUST_SOURCES:
        section_fns = sorted(
            fn_name for fn_name in runtime_func_names if parsed_funcs[fn_name].source_file == spec.filename
        )
        if not section_fns:
            continue
        out.extend(
            [
                "",
                "",
                "# ---------------------------------------------------------------------------",
                f"# {spec.section_title} (`{spec.filename}`)",
                "# ---------------------------------------------------------------------------",
                "",
            ]
        )
        for fn_name in section_fns:
            fspec = parsed_funcs[fn_name]
            fn_obj = getattr(native, fn_name)
            sig = inspect.signature(fn_obj)
            fparams = _format_param_list(sig, fspec.params)
            out.extend(_emit_def_lines(fn_name, fparams, fspec.return_type))

    out.append("")
    return "\n".join(out)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Generate or verify src/satya_py/_native.pyi.")
    parser.add_argument(
        "--check",
        action="store_true",
        help="Verify src/satya_py/_native.pyi matches generated output without writing.",
    )
    args = parser.parse_args(argv)

    stub_path = _repo_root() / "src" / "satya_py" / "_native.pyi"
    generated = generate_native_stub()

    if args.check:
        if not stub_path.exists():
            print(f"Missing stub file: {stub_path}", file=sys.stderr)
            return 1
        existing = stub_path.read_text(encoding="utf-8")
        if existing != generated:
            diff = "".join(
                difflib.unified_diff(
                    existing.splitlines(keepends=True),
                    generated.splitlines(keepends=True),
                    fromfile=str(stub_path),
                    tofile="<generated>",
                )
            )
            print(
                "src/satya_py/_native.pyi is out of date. Run `pixi run gen-stubs`.\n" + diff[:4000],
                file=sys.stderr,
            )
            return 1
        print(f"OK: {stub_path} matches compiled satya_py._native and Rust sources.")
        return 0

    with stub_path.open("w", encoding="utf-8", newline="\n") as fh:
        fh.write(generated)
    print(f"Wrote {stub_path} ({len(generated.splitlines())} lines).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
