
use super::ast;

use serde::{Serialize, Deserialize};
use std::collections::hash_map::*;
use std::hash::{Hash, Hasher};

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum Value {
    Nil,
    Number(f64),
    Bool(bool)
}

#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub enum ByteOp {
    Load(u16),

    True,
    False,
    Nil,

    Add,
    Sub,
    Mul,
    Div,

    Not,

    Equal,
    Greater,
    Less,

    Negate,

    ScopeOpen,
    ScopeClose,

    DefVar(u64),
    SetVar(u64),
    GetVar(u64),

    Jump(i16),
    JumpFalse(i16),
    JumpTrue(i16),
    Pop,
    Return
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Bytecode {
    constants: Vec<Value>,
    ops: Vec<ByteOp>
}

impl Bytecode {
    pub fn new(ast: ast::Ast) -> Result<Bytecode, String> {
        let mut bc = Bytecode {
            constants: Vec::new(),
            ops: Vec::new()
        };
        ast_to_bytecode(&mut bc, ast)?;
        Ok(bc)
    }
}

fn hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

fn ast_to_bytecode(bc: &mut Bytecode, ast: ast::Ast) -> Result<(), String> {
    match ast {
        ast::Ast::Identifier(ident) => {
            bc.ops.push(ByteOp::GetVar(hash(&ident)));
        },
        ast::Ast::Number(n) => {
            let mut idx: usize = 0;
            for val in &bc.constants {
                if let Value::Number(num) = val {
                    if n == *num {
                        bc.ops.push(ByteOp::Load(idx as u16));
                        return Ok(());
                    }
                }
                idx += 1;
            }
            bc.constants.push(Value::Number(n));
            bc.ops.push(ByteOp::Load((bc.constants.len() - 1) as u16));
        },
        // ast::Ast::String(_) => {},
        ast::Ast::Bool(b) => {
            bc.ops.push(if b { ByteOp::True } else { ByteOp::False });
        },
        ast::Ast::Statement(expr) => {
            ast_to_bytecode(bc, *expr)?;
            bc.ops.push(ByteOp::Pop);
        },
        ast::Ast::Binary(op, l, r) => {
            if let ast::BinaryOperation::Assign = op {
                if let ast::Ast::Identifier(ident) = *l {
                    ast_to_bytecode(bc, *r)?;
                    bc.ops.push(ByteOp::SetVar(hash(&ident)));
                } else {
                    return Err("Assign must assign to variable".to_string())
                }
            } else {
                ast_to_bytecode(bc, *l)?;
                ast_to_bytecode(bc, *r)?;
                match op {
                    ast::BinaryOperation::Add => bc.ops.push(ByteOp::Add),
                    ast::BinaryOperation::Subtract => bc.ops.push(ByteOp::Sub),
                    ast::BinaryOperation::Multiply => bc.ops.push(ByteOp::Mul),
                    ast::BinaryOperation::Divide => bc.ops.push(ByteOp::Div),

                    ast::BinaryOperation::Less => bc.ops.push(ByteOp::Less),
                    ast::BinaryOperation::LessEqual => {
                        bc.ops.push(ByteOp::Greater);
                        bc.ops.push(ByteOp::Not);
                    },
                    ast::BinaryOperation::Greater => bc.ops.push(ByteOp::Greater),
                    ast::BinaryOperation::GreaterEqual => {
                        bc.ops.push(ByteOp::Less);
                        bc.ops.push(ByteOp::Not);
                    },
                    ast::BinaryOperation::Equals => bc.ops.push(ByteOp::Equal),
                    ast::BinaryOperation::Assign => {},
                };
            }
        },
        ast::Ast::Unary(op, expr) => {
            ast_to_bytecode(bc, *expr)?;
            match op {
                ast::UnaryOperation::Negate => bc.ops.push(ByteOp::Negate),
                ast::UnaryOperation::Not => bc.ops.push(ByteOp::Not)
            }
        },
        ast::Ast::Return(expr) => {
            ast_to_bytecode(bc, *expr)?;
            bc.ops.push(ByteOp::Return);
        },
        ast::Ast::Block(exprs) => {
            bc.ops.push(ByteOp::ScopeOpen);
            let mut idx: usize = 1;
            let len = exprs.len();
            for e in exprs {
                match e {
                    ast::Ast::Statement(expr) => {
                        if let ast::Ast::Block(_) = *expr {
                            ast_to_bytecode(bc, *expr)?;
                        } else {
                            ast_to_bytecode(bc, ast::Ast::Statement(expr))?
                        }
                    },
                    expr => {
                        if idx != len{
                            return Err("Only the last element in a block may be an expression".to_string());
                        }
                        ast_to_bytecode(bc, expr)?;
                        bc.ops.push(ByteOp::Return);
                    }
                }
                idx += 1;
            }
            bc.ops.push(ByteOp::ScopeClose);
        },
        ast::Ast::IfElse(ifcond, ifexpr, elseifs, elseexpr) => {
            ast_to_bytecode(bc, *ifcond)?;
            bc.ops.push(ByteOp::JumpFalse(1));
            let last_jump_idx = bc.ops.len() - 1;
            ast_to_bytecode(bc, *ifexpr)?;
            bc.ops[last_jump_idx] = ByteOp::JumpFalse((bc.ops.len() as isize - (last_jump_idx) as isize + 1) as i16);
            let mut last_patch_idx: Vec<usize> = Vec::new();
            if let Some(_) = elseexpr {
                bc.ops.push(ByteOp::Jump(1));
                last_patch_idx.push(bc.ops.len() - 1);
            }

            for (cond, expr) in elseifs {
                ast_to_bytecode(bc, *cond)?;
                bc.ops.push(ByteOp::JumpFalse(1));
                let last_jump_idx = bc.ops.len() - 1;
                ast_to_bytecode(bc, *expr)?;
                bc.ops[last_jump_idx] = ByteOp::JumpFalse((bc.ops.len()  as isize - (last_jump_idx) as isize + 1) as i16);
                if let Some(_) = elseexpr {
                    bc.ops.push(ByteOp::Jump(1));
                    last_patch_idx.push(bc.ops.len() - 1);
                }
            }

            if let Some(eexpr) = elseexpr {
                ast_to_bytecode(bc, *eexpr)?;
            }

            for patch in last_patch_idx {
                bc.ops[patch] = ByteOp::Jump((bc.ops.len()  as isize - (patch) as isize) as i16);
            }
        },
        ast::Ast::While(cond, expr) => {
            let begin_idx = bc.ops.len();
            ast_to_bytecode(bc, *cond)?;
            bc.ops.push(ByteOp::JumpFalse(1));
            let cond_jump_idx = bc.ops.len() - 1;
            ast_to_bytecode(bc, *expr)?;
            bc.ops.push(ByteOp::Jump((begin_idx as isize - bc.ops.len() as isize) as i16));
            bc.ops[cond_jump_idx] = ByteOp::JumpFalse((bc.ops.len() as isize - cond_jump_idx as isize) as i16);
        },
        ast::Ast::Let(var_sig, _mutable, set_expr) => {
            if let Some(se) = set_expr {
                ast_to_bytecode(bc, *se)?;
            }
            bc.ops.push(ByteOp::DefVar(hash(&var_sig.name)));
        }
        other => { return Err(format!("Non implemented AST node {:?}", other)); }
    };
    Ok(())
}

struct Scope {
    variables: HashMap<u64, Value>
}

pub struct StackVm {
    ip_idx: usize,
    stack: Vec<Value>,
    scopes: Vec<Scope>
}

impl<'a> StackVm {
    pub fn new() -> StackVm {
        StackVm {
            ip_idx: 0,
            stack: Vec::new(),
            scopes: Vec::new(),
        }
    }

    fn stack_peek(&self, distance: usize) -> Value {
        self.stack[self.stack.len() - 1 - distance]
    }

    fn var_in_scopes_mut(scope_stack: &'a mut Vec<Scope>, id: &u64) -> Option<&'a mut Value> {
        for s in scope_stack.iter_mut().rev() {
            match s.variables.get_mut(id) {
                Some(var) => return Some(var),
                None => continue
            }
        }
        None
    }

    fn var_in_scopes(scope_stack: &'a Vec<Scope>, id: &u64) -> Option<&'a Value> {
        for s in scope_stack.iter().rev() {
            match s.variables.get(id) {
                Some(var) => return Some(var),
                None => continue
            }
        }
        None
    }

    pub fn run(&mut self, bc: Bytecode) -> Result<Value, String> {
        'run: loop {
            // println!("ip_idx: {}\nStack: {:?}\nOp: {:?}", self.ip_idx, self.stack, bc.ops.get(self.ip_idx));
            match bc.ops.get(self.ip_idx) {
                Some(ByteOp::Load(n)) => {
                    self.stack.push(bc.constants[*n as usize]);
                },
                Some(ByteOp::True) => {
                    self.stack.push(Value::Bool(true));
                },
                Some(ByteOp::False) => {
                    self.stack.push(Value::Bool(false));
                },
                Some(ByteOp::Nil) => {
                    self.stack.push(Value::Nil);
                },
                Some(ByteOp::Add) => {
                    match self.stack_peek(0) {
                        Value::Nil => { return Err("Binary add value cannot be nil".to_string()); }
                        Value::Bool(_) => { return Err("Binary add value cannot be bool".to_string()); }
                        Value::Number(_) => {
                            match self.stack_peek(1) {
                                Value::Nil => { return Err("Binary add value cannot be nil".to_string()); }
                                Value::Bool(_) => { return Err("Binary add value cannot be bool".to_string()); }
                                Value::Number(_) => {
                                    if let Some(Value::Number(b)) = self.stack.pop() {
                                        if let Some(Value::Number(a)) = self.stack.pop() {
                                            self.stack.push(Value::Number(a + b));
                                        } else {
                                            return Err("Failed to pop binary add a".to_string());
                                        }
                                    } else {
                                        return Err("Failed to pop binary add b".to_string());
                                    }

                                } 
                            }
                        }
                    }
                },
                Some(ByteOp::Sub) => {
                    match self.stack_peek(0) {
                        Value::Nil => { return Err("Binary sub value cannot be nil".to_string()); }
                        Value::Bool(_) => { return Err("Binary sub value cannot be bool".to_string()); }
                        Value::Number(_) => {
                            match self.stack_peek(1) {
                                Value::Nil => { return Err("Binary sub value cannot be nil".to_string()); }
                                Value::Bool(_) => { return Err("Binary sub value cannot be bool".to_string()); }
                                Value::Number(_) => {
                                    if let Some(Value::Number(b)) = self.stack.pop() {
                                        if let Some(Value::Number(a)) = self.stack.pop() {
                                            self.stack.push(Value::Number(a - b));
                                        } else {
                                            return Err("Failed to pop binary sub a".to_string());
                                        }
                                    } else {
                                        return Err("Failed to pop binary sub b".to_string());
                                    }

                                } 
                            }
                        }
                    }
                },
                Some(ByteOp::Mul) => {
                    match self.stack_peek(0) {
                        Value::Nil => { return Err("Binary mul value cannot be nil".to_string()); }
                        Value::Bool(_) => { return Err("Binary mul value cannot be bool".to_string()); }
                        Value::Number(_) => {
                            match self.stack_peek(1) {
                                Value::Nil => { return Err("Binary mul value cannot be nil".to_string()); }
                                Value::Bool(_) => { return Err("Binary mul value cannot be bool".to_string()); }
                                Value::Number(_) => {
                                    if let Some(Value::Number(b)) = self.stack.pop() {
                                        if let Some(Value::Number(a)) = self.stack.pop() {
                                            self.stack.push(Value::Number(a * b));
                                        } else {
                                            return Err("Failed to pop binary mul a".to_string());
                                        }
                                    } else {
                                        return Err("Failed to pop binary mul b".to_string());
                                    }

                                } 
                            }
                        }
                    }
                },
                Some(ByteOp::Div) => {
                    match self.stack_peek(0) {
                        Value::Nil => { return Err("Binary div value cannot be nil".to_string()); }
                        Value::Bool(_) => { return Err("Binary div value cannot be bool".to_string()); }
                        Value::Number(_) => {
                            match self.stack_peek(1) {
                                Value::Nil => { return Err("Binary div value cannot be nil".to_string()); }
                                Value::Bool(_) => { return Err("Binary div value cannot be bool".to_string()); }
                                Value::Number(_) => {
                                    if let Some(Value::Number(b)) = self.stack.pop() {
                                        if let Some(Value::Number(a)) = self.stack.pop() {
                                            self.stack.push(Value::Number(a / b));
                                        } else {
                                            return Err("Failed to pop binary div a".to_string());
                                        }
                                    } else {
                                        return Err("Failed to pop binary div b".to_string());
                                    }

                                } 
                            }
                        }
                    }
                },
                Some(ByteOp::Not) => {
                    match self.stack_peek(0) {
                        Value::Nil => { return Err("Unary not value cannot be nil".to_string()); }
                        Value::Bool(_) => {
                            if let Some(Value::Bool(b)) = self.stack.pop() {
                                self.stack.push(Value::Bool(!b));
                            } else {
                                return Err("Failed to pop unary not value".to_string());
                            }
                        }
                        Value::Number(_) => { return Err("Unary value cannot be number".to_string()); }
                    }
                },
                Some(ByteOp::Equal) => {
                    match self.stack_peek(0) {
                        Value::Nil => { return Err("Binary equal value cannot be nil".to_string()); }
                        Value::Bool(_) => { return Err("Binary equal value cannot be bool".to_string()); }
                        Value::Number(_) => {
                            match self.stack_peek(1) {
                                Value::Nil => { return Err("Binary equal value cannot be nil".to_string()); }
                                Value::Bool(_) => { return Err("Binary equal value cannot be bool".to_string()); }
                                Value::Number(_) => {
                                    if let Some(Value::Number(b)) = self.stack.pop() {
                                        if let Some(Value::Number(a)) = self.stack.pop() {
                                            self.stack.push(Value::Bool(a == b));
                                        } else {
                                            return Err("Failed to pop binary equal a".to_string());
                                        }
                                    } else {
                                        return Err("Failed to pop binary equal b".to_string());
                                    }

                                } 
                            }
                        }
                    }
                },
                Some(ByteOp::Greater) => {
                    match self.stack_peek(0) {
                        Value::Nil => { return Err("Binary greater value cannot be nil".to_string()); }
                        Value::Bool(_) => { return Err("Binary greater value cannot be bool".to_string()); }
                        Value::Number(_) => {
                            match self.stack_peek(1) {
                                Value::Nil => { return Err("Binary greater value cannot be nil".to_string()); }
                                Value::Bool(_) => { return Err("Binary greater value cannot be bool".to_string()); }
                                Value::Number(_) => {
                                    if let Some(Value::Number(b)) = self.stack.pop() {
                                        if let Some(Value::Number(a)) = self.stack.pop() {
                                            self.stack.push(Value::Bool(a > b));
                                        } else {
                                            return Err("Failed to pop binary greater a".to_string());
                                        }
                                    } else {
                                        return Err("Failed to pop binary greater b".to_string());
                                    }

                                } 
                            }
                        }
                    }
                },
                Some(ByteOp::Less) => {
                    match self.stack_peek(0) {
                        Value::Nil => { return Err("Binary less value cannot be nil".to_string()); }
                        Value::Bool(_) => { return Err("Binary less value cannot be bool".to_string()); }
                        Value::Number(_) => {
                            match self.stack_peek(1) {
                                Value::Nil => { return Err("Binary less value cannot be nil".to_string()); }
                                Value::Bool(_) => { return Err("Binary less value cannot be bool".to_string()); }
                                Value::Number(_) => {
                                    if let Some(Value::Number(b)) = self.stack.pop() {
                                        if let Some(Value::Number(a)) = self.stack.pop() {
                                            self.stack.push(Value::Bool(a < b));
                                        } else {
                                            return Err("Failed to pop binary less a".to_string());
                                        }
                                    } else {
                                        return Err("Failed to pop binary less b".to_string());
                                    }

                                } 
                            }
                        }
                    }
                },
                Some(ByteOp::Negate) => {
                    match self.stack_peek(0) {
                        Value::Nil => { return Err("Unary negate value cannot be nil".to_string()); }
                        Value::Bool(_) => { return Err("Unary negate value cannot be bool".to_string()); }
                        Value::Number(_) => {
                            if let Some(Value::Number(n)) = self.stack.pop() {
                                self.stack.push(Value::Number(-n));
                            } else {
                                return Err("Failed to pop unary negate value".to_string());
                            }
                        }
                    }
                },
                Some(ByteOp::ScopeOpen) => {
                    self.scopes.push(Scope { variables: HashMap::new() });
                },
                Some(ByteOp::ScopeClose) => {
                    self.scopes.pop();
                },
                Some(ByteOp::DefVar(id)) => {
                    match StackVm::var_in_scopes_mut(&mut self.scopes, id) {
                        Some(_) => {
                            return Err(format!("Variable: {} arleady defined", id))
                        },
                        None => {
                            self.scopes.last_mut().unwrap().variables.insert(*id, if let Some(val) = self.stack.pop() { self.stack.push(val); val } else { Value::Nil });
                        }
                    }
                },
                Some(ByteOp::SetVar(id)) => {
                    match StackVm::var_in_scopes_mut(&mut self.scopes, id) {
                        Some(val) => {
                            if self.stack.len() > 0 {
                                *val = self.stack.pop().unwrap();
                                self.stack.push(*val);
                            } else {
                                *val = Value::Nil;
                            }
                        },
                        None => {
                            return Err(format!("Variable {} not defined", id));
                        }
                    }
                },
                Some(ByteOp::GetVar(id)) => {
                    match StackVm::var_in_scopes(&self.scopes, id) {
                        Some(val) => {
                            self.stack.push(*val);
                        },
                        None => return Err("Failed to find variable in scope".to_string())
                    }
                }
                Some(ByteOp::Jump(distance)) => {
                    self.ip_idx = (self.ip_idx as isize + *distance as isize) as usize;
                    continue 'run;
                }
                Some(ByteOp::JumpFalse(distance)) => {
                    match self.stack_peek(0) {
                        Value::Nil => { return Err("Jump on false value cannot be nil".to_string()); },
                        Value::Bool(_) => {
                            if let Some(Value::Bool(b)) = self.stack.pop() {
                                if !b {
                                    self.ip_idx = (self.ip_idx as isize + *distance as isize) as usize;
                                    continue 'run;
                                }
                            } else {
                                return Err("Failed to pop jump on false value".to_string());
                            }
                        },
                        Value::Number(_) => { return Err("Jump on false value cannot be bool".to_string()); },
                    }
                },
                Some(ByteOp::JumpTrue(distance)) => {
                    match self.stack_peek(0) {
                        Value::Nil => { return Err("Jump on true value cannot be nil".to_string()); },
                        Value::Bool(_) => {
                            if let Some(Value::Bool(b)) = self.stack.pop() {
                                if b {
                                    self.ip_idx = (self.ip_idx as isize + *distance as isize) as usize;
                                    continue 'run;
                                }
                            } else {
                                return Err("Failed to pop jump on true value".to_string());
                            }
                        },
                        Value::Number(_) => { return Err("Jump on true value cannot be bool".to_string()); },
                    }
                },
                Some(ByteOp::Pop) => {
                    self.stack.pop();
                },
                Some(ByteOp::Return) => {
                    if let Some(v) = self.stack.pop() {
                        self.scopes.pop();
                        if self.scopes.len() > 0 {
                            self.stack.push(v);
                        } else {
                            return Ok(v);
                        }
                    } else {
                        self.scopes.pop();
                        if self.scopes.len() > 0 {
                            self.stack.push(Value::Nil);
                        } else {
                            return Ok(Value::Nil);
                        }
                    }
                },
                None => { break 'run; }
            };
            self.ip_idx += 1;
        }
        Ok(Value::Nil)
    }
}