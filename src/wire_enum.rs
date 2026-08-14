//! Shared macro for serde-compatible string enums backed by nextjson.
//!
//! NextJson's derive emits externally tagged enums as `{"Variant": ...}`, which
//! differs from serde's string form for unit enums. Every wire enum in this SDK
//! therefore uses [`wire_enum!`] so the JSON wire format stays byte-identical
//! to the Zhipu API contract while staying forward compatible with unknown
//! values.

/// Declares a string-encoded enum with nextjson wire support.
///
/// Variants map to explicit wire strings. When a trailing `; _ => Variant`
/// clause is present, unknown input decodes to that variant (a serde
/// `#[serde(other)]` equivalent). Without it, unknown input decodes to a
/// generated `Other(String)` variant that preserves the raw wire value.
///
/// # Example
///
/// ```
/// use rustglm::wire_enum;
///
/// wire_enum! {
///     /// Chat roles.
///     pub enum Role {
///         System => "system",
///         User => "user",
///     }
/// }
/// ```
#[macro_export]
macro_rules! wire_enum {
    // Form with an explicit catch-all variant for unknown input.
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $($variant:ident => $wire:literal),+ $(,)?
            ; _ => $fallback:ident
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        $vis enum $name {
            $($variant,)+
        }

        impl $name {
            /// The wire string for this variant.
            pub const fn as_str(&self) -> &'static str {
                match self {
                    $(Self::$variant => $wire,)+
                }
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                match value {
                    $($wire => Self::$variant,)+
                    _ => Self::$fallback,
                }
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self::from(value.as_str())
            }
        }

        impl $crate::NsonSerialize for $name {
            fn nextencode<E: ::nextjson::FormatEncoder>(
                &self,
                encoder: &mut E,
            ) -> ::core::result::Result<(), E::Error> {
                encoder.write_str(Self::as_str(self))
            }
        }

        impl ::nextjson::NsonSchema for $name {
            const SCHEMA: ::nextjson::TypeSchema = ::nextjson::TypeSchema::Str;
        }

        impl<'de> $crate::NsonDeserialize<'de> for $name {
            fn nextdecode_into<D: ::nextjson::FormatDecoder<'de>>(
                decoder: &mut D,
                out: &mut ::nextjson::DecodeSlot<Self>,
            ) -> ::core::result::Result<(), D::Error> {
                let value = decoder.string()?.into_owned();
                match value.as_str() {
                    $($wire => out.write(Self::$variant),)+
                    _ => out.write(Self::$fallback),
                }
                Ok(())
            }
        }
    };

    // Strict form: unknown input is an error and no fallback variant exists.
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $($variant:ident => $wire:literal),+ $(,)?
            ; strict
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        $vis enum $name {
            $($variant,)+
        }

        impl $name {
            /// The wire string for this variant.
            pub const fn as_str(&self) -> &'static str {
                match self {
                    $(Self::$variant => $wire,)+
                }
            }
        }

        impl $crate::NsonSerialize for $name {
            fn nextencode<E: ::nextjson::FormatEncoder>(
                &self,
                encoder: &mut E,
            ) -> ::core::result::Result<(), E::Error> {
                encoder.write_str(Self::as_str(self))
            }
        }

        impl ::nextjson::NsonSchema for $name {
            const SCHEMA: ::nextjson::TypeSchema = ::nextjson::TypeSchema::Str;
        }

        impl<'de> $crate::NsonDeserialize<'de> for $name {
            fn nextdecode_into<D: ::nextjson::FormatDecoder<'de>>(
                decoder: &mut D,
                out: &mut ::nextjson::DecodeSlot<Self>,
            ) -> ::core::result::Result<(), D::Error> {
                let value = decoder.string()?.into_owned();
                match value.as_str() {
                    $($wire => out.write(Self::$variant),)+
                    value => {
                        return Err(<D::Error as ::nextjson::FormatError>::custom(
                            ::std::format!("unknown {} variant: {}", ::core::stringify!($name), value),
                        ))
                    }
                }
                Ok(())
            }
        }
    };

    // Form without a catch-all: unknown input becomes `Other(String)`.
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $($variant:ident => $wire:literal),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        $vis enum $name {
            $($variant,)+
            /// An unrecognized wire value, preserved verbatim.
            Other(::std::string::String),
        }

        impl $name {
            /// The wire string for this variant.
            pub fn as_str(&self) -> &str {
                match self {
                    $(Self::$variant => $wire,)+
                    Self::Other(value) => value,
                }
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                match value {
                    $($wire => Self::$variant,)+
                    value => Self::Other(value.to_owned()),
                }
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self::from(value.as_str())
            }
        }

        impl $crate::NsonSerialize for $name {
            fn nextencode<E: ::nextjson::FormatEncoder>(
                &self,
                encoder: &mut E,
            ) -> ::core::result::Result<(), E::Error> {
                encoder.write_str(Self::as_str(self))
            }
        }

        impl ::nextjson::NsonSchema for $name {
            const SCHEMA: ::nextjson::TypeSchema = ::nextjson::TypeSchema::Str;
        }

        impl<'de> $crate::NsonDeserialize<'de> for $name {
            fn nextdecode_into<D: ::nextjson::FormatDecoder<'de>>(
                decoder: &mut D,
                out: &mut ::nextjson::DecodeSlot<Self>,
            ) -> ::core::result::Result<(), D::Error> {
                let value = decoder.string()?.into_owned();
                match value.as_str() {
                    $($wire => out.write(Self::$variant),)+
                    _ => out.write(Self::Other(value)),
                }
                Ok(())
            }
        }
    };
}
