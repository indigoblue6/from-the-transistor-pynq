//! PynqCの型表現と配置規則。

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Int,
    Char,
    Void,
    Pointer(Box<Type>),
    Array {
        element: Box<Type>,
        length: usize,
    },
    Function {
        return_type: Box<Type>,
        parameters: Vec<Type>,
    },
}

impl Type {
    pub fn size(&self) -> Option<usize> {
        match self {
            Self::Int | Self::Pointer(_) => Some(4),
            Self::Char => Some(1),
            Self::Void | Self::Function { .. } => None,
            Self::Array { element, length } => {
                element.size().and_then(|size| size.checked_mul(*length))
            }
        }
    }
    pub fn alignment(&self) -> usize {
        self.size().unwrap_or(1).min(4).max(1)
    }
    pub fn decay(&self) -> Type {
        match self {
            Self::Array { element, .. } => Self::Pointer(element.clone()),
            other => other.clone(),
        }
    }
    pub fn is_integer(&self) -> bool {
        matches!(self, Self::Int | Self::Char)
    }
    pub fn is_scalar(&self) -> bool {
        self.is_integer() || matches!(self, Self::Pointer(_))
    }
    pub fn pointed(&self) -> Option<&Type> {
        if let Self::Pointer(inner) = self {
            Some(inner)
        } else {
            None
        }
    }
    pub fn assignable_from(&self, source: &Type) -> bool {
        let source = source.decay();
        self == &source
            || (self.is_integer() && source.is_integer())
            || matches!((self, source), (Self::Pointer(a), Self::Pointer(b)) if a == &b)
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int => write!(f, "int"),
            Self::Char => write!(f, "char"),
            Self::Void => write!(f, "void"),
            Self::Pointer(inner) => write!(f, "{inner}*"),
            Self::Array { element, length } => write!(f, "{element}[{length}]"),
            Self::Function {
                return_type,
                parameters,
            } => {
                write!(f, "{return_type}(")?;
                for (i, parameter) in parameters.iter().enumerate() {
                    if i != 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{parameter}")?;
                }
                write!(f, ")")
            }
        }
    }
}
