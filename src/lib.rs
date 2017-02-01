//! This crate can parse a C++ “mangled” linker symbol name into a Rust value
//! describing what the name refers to: a variable, a function, a virtual table,
//! etc. The description type implements `Display`, producing human-readable
//! text describing the mangled name. Debuggers and profilers can use this crate
//! to provide more meaningful output.
//!
//! C++ requires the compiler to choose names for linker symbols consistently
//! across compilation units, so that two compilation units that have seen the
//! same declarations can pair up definitions in one unit with references in
//! another.  Almost all platforms other than Microsoft Windows follow the
//! [Itanium C++ ABI][itanium]'s rules for this.
//!
//! [itanium]: http://mentorembedded.github.io/cxx-abi/abi.html#mangling
//!
//! For example, suppose a C++ compilation unit has the definition:
//!
//! ```c++
//! namespace space {
//!   int foo(int x, int y) { return x+y; }
//! }
//! ```
//!
//! The Itanium C++ ABI specifies that the linker symbol for that function must
//! be named `_ZN5space3fooEii`. This crate can parse that name into a Rust
//! value representing its structure. Formatting the value with `format!` or
//! `to_string` would yield the string `"space::foo(int, int)"`, which is more
//! meaningful to the C++ developer.

#![deny(missing_docs)]
#![deny(missing_debug_implementations)]
#![deny(unsafe_code)]

// The `error_chain!` macro can recurse deeply.
#![recursion_limit = "1024"]

#[macro_use]
extern crate error_chain;

#[macro_use]
mod logging;

pub mod ast;
pub mod error;
mod index_str;
mod subs;

use ast::{Demangle, Parse};
use error::{ErrorKind, Result};
use index_str::IndexStr;
use std::fmt;

/// A `Symbol` which owns the underlying storage for the mangled name.
pub type OwnedSymbol = Symbol<Vec<u8>>;

/// A `Symbol` which borrows the underlying storage for the mangled name.
pub type BorrowedSymbol<'a> = Symbol<&'a [u8]>;

/// A mangled symbol that has been parsed into an AST.
///
/// This is generic over some storage type `T` which can be either owned or
/// borrowed. See the `OwnedSymbol` and `BorrowedSymbol` type aliases.
#[derive(Clone, Debug, PartialEq)]
pub struct Symbol<T> {
    raw: T,
    substitutions: subs::SubstitutionTable,
    parsed: ast::MangledName,
}

impl<T> Symbol<T>
    where T: AsRef<[u8]>
{
    /// Given some raw storage, parse the mangled symbol from it.
    ///
    /// ```
    /// use cpp_demangle::Symbol;
    ///
    /// let mangled = b"_ZN5space3fooEibc";
    ///
    /// let sym = Symbol::new(mangled)
    ///     .expect("Could not parse mangled symbol!");
    ///
    /// let demangled = format!("{}", sym);
    /// assert_eq!(demangled, "int space::foo(bool, char)");
    /// ```
    pub fn new(raw: T) -> Result<Symbol<T>> {
        let mut substitutions = subs::SubstitutionTable::new();

        let parsed = {
            let input = IndexStr::new(raw.as_ref());
            let (parsed, tail) = try!(ast::MangledName::parse(&mut substitutions, input));
            if tail.is_empty() {
                parsed
            } else {
                return Err(ErrorKind::UnexpectedText.into());
            }
        };

        Ok(Symbol {
            raw: raw,
            substitutions: substitutions,
            parsed: parsed,
        })
    }

    // TODO FITZGEN: new_with_tail
}

impl<T> fmt::Display for Symbol<T>
    where T: AsRef<[u8]>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut out = vec![];
        {
            let mut ctx = ast::DemangleContext::new(&self.substitutions,
                                                    self.raw.as_ref(),
                                                    &mut out);
            try!(self.parsed.demangle(&mut ctx).map_err(|_| fmt::Error));
        }
        write!(f, "{}", String::from_utf8_lossy(&out))
    }
}
