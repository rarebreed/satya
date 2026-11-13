//! Canonical `define_wire_enum!` declarative macro for `SCREAMING_SNAKE_CASE` wire enums.
//!
//! Generates `ALL`, `as_str()`, `code()`, `label()`, and case-insensitive alias-aware `from_code_str()`,
//! alongside `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `Serialize`, and `Deserialize`.

/// Declares a fieldless `SCREAMING_SNAKE_CASE` wire enum with `ALL`, `as_str`, `code`, `label`,
/// and case-insensitive alias-aware `from_code_str`.
#[macro_export]
macro_rules! define_wire_enum {
    (
        $(#[$meta:meta])*
        pub enum $Name:ident [count = $Count:expr] {
            $(
                $(#[$vmeta:meta])*
                $Variant:ident => ($code:literal, $label:literal $(, aliases: [$($alias:literal),* $(,)?])?)
            ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
        pub enum $Name {
            $(
                $(#[$vmeta])*
                $Variant,
            )+
        }

        impl $Name {
            pub const ALL: [Self; $Count] = [$(Self::$Variant),+];

            #[must_use]
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$Variant => $code),+
                }
            }

            #[must_use]
            pub const fn code(self) -> &'static str {
                self.as_str()
            }

            #[must_use]
            pub const fn label(self) -> &'static str {
                match self {
                    $(Self::$Variant => $label),+
                }
            }

            /// Parses a case-insensitive wire code or alias into the enum variant.
            #[must_use]
            pub fn from_code_str(raw: &str) -> Option<Self> {
                let trimmed = raw.trim();
                $(
                    if trimmed.eq_ignore_ascii_case($code)
                        $($(|| trimmed.eq_ignore_ascii_case($alias))*)?
                    {
                        return Some(Self::$Variant);
                    }
                )+
                None
            }
        }
    };
}

#[cfg(test)]
mod tests {
    define_wire_enum! {
        #[derive(Default)]
        pub enum SampleWireKind [count = 2] {
            #[default]
            Alpha => ("ALPHA", "Alpha label"),
            Beta => ("BETA", "Beta label", aliases: ["B_ALIAS", "SECOND"]),
        }
    }

    #[test]
    fn wire_enum_macro_supports_codes_aliases_default_and_serde() {
        assert_eq!(SampleWireKind::ALL, [SampleWireKind::Alpha, SampleWireKind::Beta]);
        assert_eq!(SampleWireKind::default(), SampleWireKind::Alpha);
        assert_eq!(SampleWireKind::Alpha.as_str(), "ALPHA");
        assert_eq!(SampleWireKind::Beta.code(), "BETA");
        assert_eq!(SampleWireKind::Beta.label(), "Beta label");
        assert_eq!(SampleWireKind::from_code_str(" alpha "), Some(SampleWireKind::Alpha));
        assert_eq!(SampleWireKind::from_code_str("b_alias"), Some(SampleWireKind::Beta));
        assert_eq!(SampleWireKind::from_code_str("SECOND"), Some(SampleWireKind::Beta));
        assert_eq!(SampleWireKind::from_code_str("unknown"), None);
        let wire = serde_json::to_string(&SampleWireKind::Beta).unwrap();
        assert_eq!(wire, "\"BETA\"");
        assert_eq!(serde_json::from_str::<SampleWireKind>(&wire).unwrap(), SampleWireKind::Beta);
    }
}
