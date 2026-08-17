//! JVM generic-signature parsing (JVMS §4.7.9.1).
//!
//! The `Signature` attribute (§4.7.9) records the *generic* type information a
//! `.class` file would otherwise erase: type parameters (`<T:...>`), parameterized
//! supertypes (`Ljava/util/List<Ljava/lang/String;>;`), type-variable references
//! (`TT;`), wildcards, and generic arrays. The payload is a single UTF-8 string
//! whose grammar is defined in JVMS §4.7.9.1 — a language distinct from the plain
//! field/method *descriptor* grammar.
//!
//! # Why this lives in `duke-classfile`
//!
//! A generic signature is a **class-file-format artifact**: it is produced by the
//! compiler, stored verbatim in the constant pool, and its grammar is specified
//! alongside the rest of the class-file format. Parsing it into a typed AST here
//! (rather than in the interpreter) keeps the interpreter thin — the interpreter
//! consumes a structured [`ClassSignature`] / [`TypeSignature`] rather than walking
//! raw characters — and it lets the grammar be unit-tested in isolation, without
//! spinning up a heap or a class loader.
//!
//! # Robustness
//!
//! Every entry point returns a [`Result`]. The parser NEVER panics on malformed
//! input: a truncated string, an unexpected character, or an unterminated type
//! argument yields `Err(SignatureError)` rather than an index-out-of-bounds. This
//! matters because signatures come from untrusted `.class` bytes.

use std::fmt;

/// Error produced when a generic signature does not conform to JVMS §4.7.9.1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureError {
    /// Human-readable reason.
    pub message: String,
    /// Byte offset within the signature at which parsing failed.
    pub position: usize,
}

impl fmt::Display for SignatureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid generic signature at byte {}: {}",
            self.position, self.message
        )
    }
}

impl std::error::Error for SignatureError {}

/// A parsed `TypeParameter` (JVMS §4.7.9.1): a name plus its bounds.
///
/// e.g. `T:Ljava/lang/Object;` → `{ name: "T", bounds: [Class(java/lang/Object)] }`.
/// The class bound may be omitted in the source (`T::Lfoo/Bar;`), in which case it
/// contributes nothing and only the interface bounds appear.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeParam {
    /// The type-variable's simple name (e.g. `T`, `E`, `K`).
    pub name: String,
    /// Ordered class + interface bounds. Empty if the only bound was the implicit
    /// `Object` class bound elided in the signature.
    pub bounds: Vec<TypeSignature>,
}

/// A parsed `ClassSignature` (JVMS §4.7.9.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassSignature {
    /// Ordered formal type parameters (`<...>`); empty when the class is not generic.
    pub type_params: Vec<TypeParam>,
    /// The (possibly parameterized) direct superclass.
    pub super_class: ClassTypeSignature,
    /// The (possibly parameterized) directly-implemented interfaces, in order.
    pub interfaces: Vec<ClassTypeSignature>,
}

/// A parsed `MethodSignature` (JVMS §4.7.9.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodSignature {
    /// Ordered formal type parameters (`<...>`); empty when the method is not generic.
    pub type_params: Vec<TypeParam>,
    /// Ordered parameter type signatures.
    pub parameters: Vec<TypeSignature>,
    /// The return type (`TypeSignature::Void` for `V`).
    pub return_type: TypeSignature,
    /// Ordered `throws` clauses (each a class or type-variable signature).
    pub throws: Vec<TypeSignature>,
}

/// One argument inside a `TypeArguments` list (`<...>`), JVMS §4.7.9.1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeArgument {
    /// Unbounded wildcard `*` (`<?>`).
    Wildcard,
    /// An exact reference type (`<Ljava/lang/String;>`).
    Exact(TypeSignature),
    /// Upper-bounded wildcard `+Bound` (`<? extends Bound>`).
    Extends(TypeSignature),
    /// Lower-bounded wildcard `-Bound` (`<? super Bound>`).
    Super(TypeSignature),
}

/// A (possibly parameterized) class type signature, JVMS §4.7.9.1
/// (`ClassTypeSignature`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassTypeSignature {
    /// The binary internal name (`/`-separated package + `$`-joined nested classes),
    /// e.g. `java/util/Map` or `java/util/Map$Entry`.
    pub name: String,
    /// Type arguments applied to the outermost simple class type signature. Empty
    /// for a raw / non-generic reference.
    ///
    /// SIMPLIFICATION: type arguments attached to *inner*-class suffixes
    /// (`Louter/A<X>.Inner<Y>;`) are not tracked separately — the inner name is
    /// folded into [`name`](Self::name) with `$`, and only the outermost argument
    /// list is retained. This is sufficient for the generic surface Duke models
    /// (`getTypeParameters`, `getGenericSuperclass`, `getGenericInterfaces`).
    pub type_arguments: Vec<TypeArgument>,
}

/// A parsed `JavaTypeSignature` / `ReferenceTypeSignature`, JVMS §4.7.9.1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeSignature {
    /// A primitive base type: one of `B C D F I J S Z`.
    Primitive(char),
    /// The void result type `V` (only legal as a method return type).
    Void,
    /// A type-variable reference `TIdentifier;` (carries the identifier).
    TypeVariable(String),
    /// An array type `[ComponentSignature` (JVMS `ArrayTypeSignature`).
    Array(Box<Self>),
    /// A class type signature `Lname<args>;`.
    Class(ClassTypeSignature),
}

/// Parse a class-level generic signature (JVMS §4.7.9.1 `ClassSignature`).
///
/// # Errors
/// Returns [`SignatureError`] if `input` is not a well-formed class signature.
pub fn parse_class_signature(input: &str) -> Result<ClassSignature, SignatureError> {
    let mut p = Parser::new(input);
    let sig = p.parse_class_signature()?;
    p.expect_end()?;
    Ok(sig)
}

/// Parse a field-level generic signature (JVMS §4.7.9.1 `FieldSignature`), i.e. a
/// single `ReferenceTypeSignature`.
///
/// # Errors
/// Returns [`SignatureError`] if `input` is not a well-formed field signature.
pub fn parse_field_signature(input: &str) -> Result<TypeSignature, SignatureError> {
    let mut p = Parser::new(input);
    let sig = p.parse_reference_type_signature()?;
    p.expect_end()?;
    Ok(sig)
}

/// Parse a method-level generic signature (JVMS §4.7.9.1 `MethodSignature`).
///
/// # Errors
/// Returns [`SignatureError`] if `input` is not a well-formed method signature.
pub fn parse_method_signature(input: &str) -> Result<MethodSignature, SignatureError> {
    let mut p = Parser::new(input);
    let sig = p.parse_method_signature()?;
    p.expect_end()?;
    Ok(sig)
}

/// Recursive-descent cursor over the raw signature bytes.
///
/// Byte-level scanning is sound: every grammar delimiter (`. ; [ / < > : ^ ( ) * + -`
/// and the type tags) is a 7-bit ASCII character, and UTF-8 continuation bytes of a
/// multi-byte identifier character are always `>= 0x80`, so they can never be
/// mistaken for a delimiter.
struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
    depth: usize,
}

impl<'a> Parser<'a> {
    const fn new(input: &'a str) -> Self {
        Self {
            bytes: input.as_bytes(),
            pos: 0,
            depth: 0,
        }
    }

    fn check_depth<T, F>(&mut self, f: F) -> Result<T, SignatureError>
    where
        F: FnOnce(&mut Self) -> Result<T, SignatureError>,
    {
        if self.depth >= 256 {
            return self.err("signature recursion depth limit exceeded");
        }
        self.depth += 1;
        let res = f(self);
        self.depth -= 1;
        res
    }

    fn err<T>(&self, message: impl Into<String>) -> Result<T, SignatureError> {
        Err(SignatureError {
            message: message.into(),
            position: self.pos,
        })
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let b = self.bytes.get(self.pos).copied();
        if b.is_some() {
            self.pos += 1;
        }
        b
    }

    fn expect(&mut self, expected: u8) -> Result<(), SignatureError> {
        match self.peek() {
            Some(b) if b == expected => {
                self.pos += 1;
                Ok(())
            }
            Some(b) => self.err(format!(
                "expected '{}' but found '{}'",
                expected as char, b as char
            )),
            None => self.err(format!("expected '{}' but reached end", expected as char)),
        }
    }

    fn expect_end(&self) -> Result<(), SignatureError> {
        if self.pos == self.bytes.len() {
            Ok(())
        } else {
            Err(SignatureError {
                message: format!("trailing characters after signature (at byte {})", self.pos),
                position: self.pos,
            })
        }
    }

    /// Read an `Identifier`: a run of characters up to (but not consuming) one of
    /// the terminating delimiters `. ; [ / < > :`.
    fn parse_identifier(&mut self) -> Result<String, SignatureError> {
        let start = self.pos;
        while let Some(b) = self.peek() {
            match b {
                b'.' | b';' | b'[' | b'/' | b'<' | b'>' | b':' => break,
                _ => self.pos += 1,
            }
        }
        if self.pos == start {
            return self.err("empty identifier");
        }
        Ok(String::from_utf8_lossy(&self.bytes[start..self.pos]).into_owned())
    }

    // ---- ClassSignature -------------------------------------------------

    fn parse_class_signature(&mut self) -> Result<ClassSignature, SignatureError> {
        let type_params = self.parse_optional_type_parameters()?;
        let super_class = self.parse_class_type_signature()?;
        let mut interfaces = Vec::new();
        while self.peek() == Some(b'L') {
            interfaces.push(self.parse_class_type_signature()?);
        }
        Ok(ClassSignature {
            type_params,
            super_class,
            interfaces,
        })
    }

    // ---- MethodSignature ------------------------------------------------

    fn parse_method_signature(&mut self) -> Result<MethodSignature, SignatureError> {
        let type_params = self.parse_optional_type_parameters()?;
        self.expect(b'(')?;
        let mut parameters = Vec::new();
        while self.peek() != Some(b')') {
            if self.peek().is_none() {
                return self.err("unterminated method parameter list");
            }
            parameters.push(self.parse_type_signature()?);
        }
        self.expect(b')')?;
        let return_type = if self.peek() == Some(b'V') {
            self.pos += 1;
            TypeSignature::Void
        } else {
            self.parse_type_signature()?
        };
        let mut throws = Vec::new();
        while self.peek() == Some(b'^') {
            self.pos += 1;
            throws.push(self.parse_reference_type_signature()?);
        }
        Ok(MethodSignature {
            type_params,
            parameters,
            return_type,
            throws,
        })
    }

    // ---- TypeParameters -------------------------------------------------

    fn parse_optional_type_parameters(&mut self) -> Result<Vec<TypeParam>, SignatureError> {
        if self.peek() != Some(b'<') {
            return Ok(Vec::new());
        }
        self.pos += 1; // consume '<'
        let mut params = Vec::new();
        while self.peek() != Some(b'>') {
            if self.peek().is_none() {
                return self.err("unterminated type-parameter list");
            }
            params.push(self.parse_type_parameter()?);
        }
        self.expect(b'>')?;
        if params.is_empty() {
            return self.err("empty type-parameter list");
        }
        Ok(params)
    }

    fn parse_type_parameter(&mut self) -> Result<TypeParam, SignatureError> {
        let name = self.parse_identifier()?;
        let mut bounds = Vec::new();
        // ClassBound: ':' [ReferenceTypeSignature]
        self.expect(b':')?;
        if self.peek() != Some(b':') && self.class_bound_present() {
            bounds.push(self.parse_reference_type_signature()?);
        }
        // InterfaceBound*: ':' ReferenceTypeSignature
        while self.peek() == Some(b':') {
            self.pos += 1;
            bounds.push(self.parse_reference_type_signature()?);
        }
        Ok(TypeParam { name, bounds })
    }

    /// After the class-bound `:`, a reference type signature is present unless the
    /// next character starts another bound (`:`) or ends the parameter list (`>`).
    fn class_bound_present(&self) -> bool {
        !matches!(self.peek(), Some(b':' | b'>') | None)
    }

    // ---- ReferenceTypeSignature / JavaTypeSignature ---------------------

    /// `JavaTypeSignature`: a base type OR a reference type.
    fn parse_type_signature(&mut self) -> Result<TypeSignature, SignatureError> {
        match self.peek() {
            Some(b'B' | b'C' | b'D' | b'F' | b'I' | b'J' | b'S' | b'Z') => {
                let b = self.bump().unwrap();
                Ok(TypeSignature::Primitive(b as char))
            }
            Some(_) => self.parse_reference_type_signature(),
            None => self.err("expected a type signature but reached end"),
        }
    }

    /// `ReferenceTypeSignature`: class, type-variable, or array.
    fn parse_reference_type_signature(&mut self) -> Result<TypeSignature, SignatureError> {
        self.check_depth(|this| match this.peek() {
            Some(b'L') => Ok(TypeSignature::Class(this.parse_class_type_signature()?)),
            Some(b'T') => this.parse_type_variable_signature(),
            Some(b'[') => this.parse_array_type_signature(),
            Some(b) => this.err(format!(
                "expected a reference type signature but found '{}'",
                b as char
            )),
            None => this.err("expected a reference type signature but reached end"),
        })
    }

    fn parse_type_variable_signature(&mut self) -> Result<TypeSignature, SignatureError> {
        self.expect(b'T')?;
        let name = self.parse_identifier()?;
        self.expect(b';')?;
        Ok(TypeSignature::TypeVariable(name))
    }

    fn parse_array_type_signature(&mut self) -> Result<TypeSignature, SignatureError> {
        self.expect(b'[')?;
        let component = self.parse_type_signature()?;
        Ok(TypeSignature::Array(Box::new(component)))
    }

    fn parse_class_type_signature(&mut self) -> Result<ClassTypeSignature, SignatureError> {
        self.expect(b'L')?;
        // PackageSpecifier* SimpleClassTypeSignature: read identifiers separated by
        // '/', where the final identifier carries the (optional) type arguments.
        let mut name = String::new();
        loop {
            let ident = self.parse_identifier()?;
            name.push_str(&ident);
            if self.peek() == Some(b'/') {
                self.pos += 1;
                name.push('/');
            } else {
                break;
            }
        }
        // TypeArguments on the outermost simple class type signature.
        let type_arguments = self.parse_optional_type_arguments()?;
        // ClassTypeSignatureSuffix*: '.' SimpleClassTypeSignature (nested classes).
        // Fold the inner name into `name` with '$'; inner type args are discarded
        // (documented simplification).
        while self.peek() == Some(b'.') {
            self.pos += 1;
            let inner = self.parse_identifier()?;
            name.push('$');
            name.push_str(&inner);
            let _inner_args = self.parse_optional_type_arguments()?;
        }
        self.expect(b';')?;
        Ok(ClassTypeSignature {
            name,
            type_arguments,
        })
    }

    fn parse_optional_type_arguments(&mut self) -> Result<Vec<TypeArgument>, SignatureError> {
        if self.peek() != Some(b'<') {
            return Ok(Vec::new());
        }
        self.pos += 1; // consume '<'
        let mut args = Vec::new();
        while self.peek() != Some(b'>') {
            if self.peek().is_none() {
                return self.err("unterminated type-argument list");
            }
            args.push(self.parse_type_argument()?);
        }
        self.expect(b'>')?;
        if args.is_empty() {
            return self.err("empty type-argument list");
        }
        Ok(args)
    }

    fn parse_type_argument(&mut self) -> Result<TypeArgument, SignatureError> {
        match self.peek() {
            Some(b'*') => {
                self.pos += 1;
                Ok(TypeArgument::Wildcard)
            }
            Some(b'+') => {
                self.pos += 1;
                Ok(TypeArgument::Extends(
                    self.parse_reference_type_signature()?,
                ))
            }
            Some(b'-') => {
                self.pos += 1;
                Ok(TypeArgument::Super(self.parse_reference_type_signature()?))
            }
            Some(_) => Ok(TypeArgument::Exact(self.parse_reference_type_signature()?)),
            None => self.err("expected a type argument but reached end"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn class(name: &str) -> TypeSignature {
        TypeSignature::Class(ClassTypeSignature {
            name: name.to_string(),
            type_arguments: Vec::new(),
        })
    }

    #[test]
    fn single_unbounded_type_param() {
        let sig = parse_class_signature("<T:Ljava/lang/Object;>Ljava/lang/Object;").unwrap();
        assert_eq!(sig.type_params.len(), 1);
        assert_eq!(sig.type_params[0].name, "T");
        assert_eq!(sig.type_params[0].bounds, vec![class("java/lang/Object")]);
        assert_eq!(sig.super_class.name, "java/lang/Object");
        assert!(sig.interfaces.is_empty());
    }

    #[test]
    fn non_generic_class_signature() {
        // A class that merely has a parameterized supertype but no own type params.
        let sig = parse_class_signature("Ljava/lang/Object;Ljava/util/List<Ljava/lang/String;>;")
            .unwrap();
        assert!(sig.type_params.is_empty());
        assert_eq!(sig.super_class.name, "java/lang/Object");
        assert_eq!(sig.interfaces.len(), 1);
        assert_eq!(sig.interfaces[0].name, "java/util/List");
        assert_eq!(sig.interfaces[0].type_arguments.len(), 1);
        assert_eq!(
            sig.interfaces[0].type_arguments[0],
            TypeArgument::Exact(class("java/lang/String"))
        );
    }

    #[test]
    fn multiple_bounded_type_params() {
        // <K:Ljava/lang/Object;V:Ljava/lang/Object;>Ljava/lang/Object;
        let sig =
            parse_class_signature("<K:Ljava/lang/Object;V:Ljava/lang/Object;>Ljava/lang/Object;")
                .unwrap();
        assert_eq!(sig.type_params.len(), 2);
        assert_eq!(sig.type_params[0].name, "K");
        assert_eq!(sig.type_params[1].name, "V");
    }

    #[test]
    fn type_param_with_interface_bounds_and_elided_class_bound() {
        // <T::Ljava/lang/Comparable<TT;>;>Ljava/lang/Object;
        // The class bound is elided (`::`), only an interface bound is present.
        let sig =
            parse_class_signature("<T::Ljava/lang/Comparable<TT;>;>Ljava/lang/Object;").unwrap();
        assert_eq!(sig.type_params.len(), 1);
        let tp = &sig.type_params[0];
        assert_eq!(tp.name, "T");
        assert_eq!(tp.bounds.len(), 1);
        match &tp.bounds[0] {
            TypeSignature::Class(c) => {
                assert_eq!(c.name, "java/lang/Comparable");
                assert_eq!(
                    c.type_arguments,
                    vec![TypeArgument::Exact(TypeSignature::TypeVariable(
                        "T".to_string()
                    ))]
                );
            }
            other => panic!("expected class bound, got {other:?}"),
        }
    }

    #[test]
    fn type_param_with_class_and_interface_bounds() {
        // <T:Ljava/lang/Number;:Ljava/io/Serializable;>Ljava/lang/Object;
        let sig = parse_class_signature(
            "<T:Ljava/lang/Number;:Ljava/io/Serializable;>Ljava/lang/Object;",
        )
        .unwrap();
        let tp = &sig.type_params[0];
        assert_eq!(tp.bounds.len(), 2);
        assert_eq!(tp.bounds[0], class("java/lang/Number"));
        assert_eq!(tp.bounds[1], class("java/io/Serializable"));
    }

    #[test]
    fn parameterized_superclass() {
        // class Foo extends AbstractList<String>
        let sig = parse_class_signature("Ljava/util/AbstractList<Ljava/lang/String;>;").unwrap();
        assert_eq!(sig.super_class.name, "java/util/AbstractList");
        assert_eq!(
            sig.super_class.type_arguments,
            vec![TypeArgument::Exact(class("java/lang/String"))]
        );
    }

    #[test]
    fn nested_generics() {
        let sig = parse_field_signature("Ljava/util/List<Ljava/util/List<Ljava/lang/String;>;>;")
            .unwrap();
        match sig {
            TypeSignature::Class(outer) => {
                assert_eq!(outer.name, "java/util/List");
                match &outer.type_arguments[0] {
                    TypeArgument::Exact(TypeSignature::Class(inner)) => {
                        assert_eq!(inner.name, "java/util/List");
                        assert_eq!(
                            inner.type_arguments[0],
                            TypeArgument::Exact(class("java/lang/String"))
                        );
                    }
                    other => panic!("expected inner list, got {other:?}"),
                }
            }
            other => panic!("expected class, got {other:?}"),
        }
    }

    #[test]
    fn wildcards() {
        // Map<? extends Number, ? super Integer>
        let sig = parse_field_signature("Ljava/util/Map<+Ljava/lang/Number;-Ljava/lang/Integer;>;")
            .unwrap();
        match sig {
            TypeSignature::Class(c) => {
                assert_eq!(
                    c.type_arguments,
                    vec![
                        TypeArgument::Extends(class("java/lang/Number")),
                        TypeArgument::Super(class("java/lang/Integer")),
                    ]
                );
            }
            other => panic!("expected class, got {other:?}"),
        }
    }

    #[test]
    fn unbounded_wildcard() {
        let sig = parse_field_signature("Ljava/lang/Class<*>;").unwrap();
        match sig {
            TypeSignature::Class(c) => {
                assert_eq!(c.type_arguments, vec![TypeArgument::Wildcard]);
            }
            other => panic!("expected class, got {other:?}"),
        }
    }

    #[test]
    fn type_variable_argument() {
        let sig = parse_field_signature("Ljava/util/List<TT;>;").unwrap();
        match sig {
            TypeSignature::Class(c) => {
                assert_eq!(
                    c.type_arguments,
                    vec![TypeArgument::Exact(TypeSignature::TypeVariable(
                        "T".to_string()
                    ))]
                );
            }
            other => panic!("expected class, got {other:?}"),
        }
    }

    #[test]
    fn array_field_signature() {
        // List<String>[]
        let sig = parse_field_signature("[Ljava/util/List<Ljava/lang/String;>;").unwrap();
        match sig {
            TypeSignature::Array(inner) => match *inner {
                TypeSignature::Class(c) => assert_eq!(c.name, "java/util/List"),
                other => panic!("expected class element, got {other:?}"),
            },
            other => panic!("expected array, got {other:?}"),
        }
    }

    #[test]
    fn generic_array_of_type_variable() {
        // T[]
        let sig = parse_field_signature("[TT;").unwrap();
        assert_eq!(
            sig,
            TypeSignature::Array(Box::new(TypeSignature::TypeVariable("T".to_string())))
        );
    }

    #[test]
    fn multidimensional_primitive_array() {
        // int[][]
        let sig = parse_field_signature("[[I").unwrap();
        assert_eq!(
            sig,
            TypeSignature::Array(Box::new(TypeSignature::Array(Box::new(
                TypeSignature::Primitive('I')
            ))))
        );
    }

    #[test]
    fn nested_inner_class_signature() {
        // Map.Entry<String, Integer>  → outer.Inner folded to $ with outer name.
        let sig = parse_field_signature(
            "Ljava/util/Map<Ljava/lang/String;Ljava/lang/Integer;>.Entry<Ljava/lang/String;Ljava/lang/Integer;>;",
        )
        .unwrap();
        match sig {
            TypeSignature::Class(c) => {
                assert_eq!(c.name, "java/util/Map$Entry");
                // Outermost args retained.
                assert_eq!(c.type_arguments.len(), 2);
            }
            other => panic!("expected class, got {other:?}"),
        }
    }

    #[test]
    fn method_signature_generic() {
        // <T:Ljava/lang/Object;>(Ljava/util/List<TT;>;I)TT;
        let sig =
            parse_method_signature("<T:Ljava/lang/Object;>(Ljava/util/List<TT;>;I)TT;").unwrap();
        assert_eq!(sig.type_params.len(), 1);
        assert_eq!(sig.type_params[0].name, "T");
        assert_eq!(sig.parameters.len(), 2);
        match &sig.parameters[0] {
            TypeSignature::Class(c) => assert_eq!(c.name, "java/util/List"),
            other => panic!("expected list param, got {other:?}"),
        }
        assert_eq!(sig.parameters[1], TypeSignature::Primitive('I'));
        assert_eq!(
            sig.return_type,
            TypeSignature::TypeVariable("T".to_string())
        );
    }

    #[test]
    fn method_signature_void_with_throws() {
        // (Ljava/lang/String;)V^Ljava/io/IOException;^TX;
        let sig =
            parse_method_signature("(Ljava/lang/String;)V^Ljava/io/IOException;^TX;").unwrap();
        assert!(sig.type_params.is_empty());
        assert_eq!(sig.parameters.len(), 1);
        assert_eq!(sig.return_type, TypeSignature::Void);
        assert_eq!(sig.throws.len(), 2);
        assert_eq!(sig.throws[0], class("java/io/IOException"));
        assert_eq!(sig.throws[1], TypeSignature::TypeVariable("X".to_string()));
    }

    #[test]
    fn recursive_bound_enum_style() {
        // Enum-style: <E:Ljava/lang/Enum<TE;>;>Ljava/lang/Object;
        let sig = parse_class_signature("<E:Ljava/lang/Enum<TE;>;>Ljava/lang/Object;").unwrap();
        assert_eq!(sig.type_params.len(), 1);
        assert_eq!(sig.type_params[0].name, "E");
        match &sig.type_params[0].bounds[0] {
            TypeSignature::Class(c) => {
                assert_eq!(c.name, "java/lang/Enum");
                assert_eq!(
                    c.type_arguments[0],
                    TypeArgument::Exact(TypeSignature::TypeVariable("E".to_string()))
                );
            }
            other => panic!("expected enum bound, got {other:?}"),
        }
    }

    // ---- Malformed input: must never panic, always Err --------------------

    #[test]
    fn malformed_truncated_type_param() {
        assert!(parse_class_signature("<T:Ljava/lang/Object").is_err());
    }

    #[test]
    fn malformed_unterminated_type_arguments() {
        assert!(parse_field_signature("Ljava/util/List<Ljava/lang/String;").is_err());
    }

    #[test]
    fn malformed_empty_input() {
        assert!(parse_class_signature("").is_err());
        assert!(parse_field_signature("").is_err());
        assert!(parse_method_signature("").is_err());
    }

    #[test]
    fn malformed_unexpected_char() {
        assert!(parse_field_signature("Q123;").is_err());
    }

    #[test]
    fn malformed_trailing_garbage() {
        assert!(parse_field_signature("Ljava/lang/String;XYZ").is_err());
    }

    #[test]
    fn malformed_missing_close_paren() {
        assert!(parse_method_signature("(Ljava/lang/String;").is_err());
    }

    #[test]
    fn never_panics_on_arbitrary_ascii() {
        // Fuzz-lite: throw a pile of signature-ish fragments at each entry point.
        let samples = [
            "<",
            ">",
            "L",
            "T",
            "[",
            ";",
            "()",
            "<>",
            "L;",
            "T;",
            "[L;",
            "<::>",
            "Ljava/util/Map<+-*>;",
            "(((",
            ")))",
            "^^^",
            "L/;",
            "<T:>",
            "TT",
            "[[[[[[",
            "Ljava/util/List<TT;>;extra",
            "<T:Ljava/lang/Object;>",
        ];
        for s in samples {
            let _ = parse_class_signature(s);
            let _ = parse_field_signature(s);
            let _ = parse_method_signature(s);
        }
    }
}
