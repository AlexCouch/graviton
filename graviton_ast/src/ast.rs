use serde::{Deserialize, Serialize};

#[repr(u8)]
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum BinaryOperation {
    Add,
    Subtract,
    Multiply,
    Divide,

    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Equal,
    NotEqual,

    And,
    Or,

    Assign,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum UnaryOperation {
    Negate,

    Not,
}

#[repr(u8)]
#[derive(Hash, Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum PrimitiveType {
    Nil,
    Bool,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
}

#[derive(Hash, Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum TypeSignature {
    Primitive(PrimitiveType),
    Function(FunctionSignature),
    Custom(String),
}

impl<'a> TypeSignature {
    pub fn new(t: &'a str) -> TypeSignature {
        match t {
            "Nil" => TypeSignature::Primitive(PrimitiveType::Nil),
            "Bool" => TypeSignature::Primitive(PrimitiveType::Bool),
            "I8" => TypeSignature::Primitive(PrimitiveType::I8),
            "I16" => TypeSignature::Primitive(PrimitiveType::I16),
            "I32" => TypeSignature::Primitive(PrimitiveType::I32),
            "I64" => TypeSignature::Primitive(PrimitiveType::I64),
            "U8" => TypeSignature::Primitive(PrimitiveType::U8),
            "U16" => TypeSignature::Primitive(PrimitiveType::U16),
            "U32" => TypeSignature::Primitive(PrimitiveType::U32),
            "U64" => TypeSignature::Primitive(PrimitiveType::U64),
            _ => TypeSignature::Custom(t.to_string()),
        }
    }
    pub fn is_number(&self) -> bool {
        match self {
            TypeSignature::Primitive(p) => match p {
                PrimitiveType::Nil | PrimitiveType::Bool => false,
                _ => true,
            },
            _ => false,
        }
    }
    pub fn is_bool(&self) -> bool {
        match self {
            TypeSignature::Primitive(p) => match p {
                PrimitiveType::Bool => true,
                _ => false,
            },
            _ => false,
        }
    }
    pub fn is_nil(&self) -> bool {
        match self {
            TypeSignature::Primitive(p) => match p {
                PrimitiveType::Nil => true,
                _ => false,
            },
            _ => false,
        }
    }
    pub fn is_signed(&self) -> bool {
        match self {
            TypeSignature::Primitive(p) => match p {
                PrimitiveType::I8
                | PrimitiveType::I16
                | PrimitiveType::I32
                | PrimitiveType::I64 => true,
                _ => false,
            },
            _ => false,
        }
    }
    pub fn is_unsigned(&self) -> bool {
        match self {
            TypeSignature::Primitive(p) => match p {
                PrimitiveType::U8
                | PrimitiveType::U16
                | PrimitiveType::U32
                | PrimitiveType::U64 => true,
                _ => false,
            },
            _ => false,
        }
    }
    pub fn is_function(&self) -> bool {
        match self {
            TypeSignature::Function(_) => true,
            _ => false,
        }
    }
    pub fn is_custom(&self) -> bool {
        match self {
            TypeSignature::Custom(_) => true,
            _ => false,
        }
    }
}

#[derive(Hash, Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct VariableSignature {
    pub mutable: bool,
    pub type_sig: Option<TypeSignature>,
}

#[derive(Hash, Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct FunctionSignature {
    pub params: Vec<VariableSignature>,
    pub return_type: Option<Box<TypeSignature>>,
}

#[macro_export]
macro_rules! make_fn_sig {
    (fn ( $( $type_:ident ),* ): $ret:ident) => {
        ast::FunctionSignature{ params: vec!(
            $(
                ast::VariableSignature { mutable: true, type_sig: Some(ast::TypeSignature::new(stringify!($type_))) },
            )*
        ), return_type: Some(Box::new(ast::TypeSignature::new(stringify!($ret)))) }
    };
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Ast {
    // identifier name
    Identifier(String),

    // number value
    Number(f64),

    // string value
    String(String),

    // boolean value
    Bool(bool),

    // expr
    Statement(Box<AstNode>),

    // operator, left expr, right expr
    Binary(BinaryOperation, Box<AstNode>, Box<AstNode>),

    // operator, expr
    Unary(UnaryOperation, Box<AstNode>),

    // returned expression
    Return(Box<AstNode>),

    // vector of expr
    Block(Vec<AstNode>),

    // if cond, if expr, else if conds, else if exprs, optional else expr
    IfElse(
        Box<AstNode>,
        Box<AstNode>,
        Vec<(Box<AstNode>, Box<AstNode>)>,
        Option<Box<AstNode>>,
    ),

    // while cond, while expr
    While(Box<AstNode>, Box<AstNode>),

    // name, variable signature, optional value expr
    Let(String, VariableSignature, Option<Box<AstNode>>),

    // import file name, file's ast
    Import(String, Box<AstNode>),

    // function parameters, param names return type, implementation
    FnDef(FunctionSignature, Vec<String>, Box<AstNode>),

    // expression that evaluates to function, arguments
    FnCall(Box<AstNode>, Vec<AstNode>),

    // expression, type to cast to
    As(Box<AstNode>, TypeSignature),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AstNode {
    pub node: Ast,
    pub pos: super::Position,
    pub type_sig: Option<TypeSignature>,
}

impl std::hash::Hash for AstNode {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.pos.hash(state);
        self.type_sig.hash(state);
    }
}

impl std::fmt::Debug for AstNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(type_sig) = &self.type_sig {
            if f.alternate() {
                write!(
                    f,
                    "[{},{}]: {:?}: {:#?}",
                    self.pos.line, self.pos.col, type_sig, self.node
                )
            } else {
                write!(
                    f,
                    "[{},{}]: {:?}: {:?}",
                    self.pos.line, self.pos.col, type_sig, self.node
                )
            }
        } else {
            if f.alternate() {
                write!(f, "[{},{}]: {:#?}", self.pos.line, self.pos.col, self.node)
            } else {
                write!(f, "[{},{}]: {:?}", self.pos.line, self.pos.col, self.node)
            }
        }
    }
}
