//! 简单的符号运算系统，用于将形状符号化。
//!
//! 考虑到形状运算的实际情况，只支持多项式的运算。

use std::{
    collections::{BTreeSet, HashMap, VecDeque},
    fmt::Display,
    ops::{Add, Div, Mul, Neg, Sub},
};
use num_rational::Ratio;

/// 形状的一个维度，或参与维度运算的值。
///
/// ```rust
/// # use std::collections::HashMap;
/// # use symbolic_expr::Expr;
/// let a = Expr::var("a");
/// let b = Expr::var("b");
/// let _1 = Expr::from(1);
/// let expr = (a + _1 - 2) * 3 / (b + 1);
/// assert_eq!(expr.substitute(&HashMap::from([("a", 8), ("b", 6)])), 3);
/// ```
#[derive(Clone, Debug)]
pub enum Expr {
    /// 常量
    Constant(usize),
    /// 变量
    Variable(String),
    /// 和式
    Sum(VecDeque<Operand>),
    /// 积式
    Product(VecDeque<Operand>),
    /// 已转换为有理式的表达式
    Rational(RationalExpression),
}

impl Default for Expr {
    fn default() -> Self {
        Self::Constant(0)
    }
}

impl Expr {
    /// 变量。
    pub fn var(symbol: impl Display) -> Self {
        Self::Variable(symbol.to_string())
    }

    /// 维度作为正操作数。
    pub fn positive(self) -> Operand {
        Operand {
            ty: Type::Positive,
            expr: self,
        }
    }

    /// 维度作为负操作数。
    pub fn negative(self) -> Operand {
        Operand {
            ty: Type::Negative,
            expr: self,
        }
    }

    /// 统计表达式中出现的变量名。
    pub fn variables(&self) -> BTreeSet<&str> {
        let mut ans = BTreeSet::new();
        self.append_variables(&mut ans);
        ans
    }

    /// 遍历表达式，递归地将变量名添加到集合。
    pub fn append_variables<'s>(&'s self, set: &mut BTreeSet<&'s str>) {
        match self {
            Self::Constant(_) => {}
            Self::Variable(name) => {
                set.insert(name);
            }
            Self::Sum(operands) | Self::Product(operands) => {
                operands.iter().for_each(|op| op.expr.append_variables(set))
            }
            Self::Rational(rational) => {
                rational.numer.iter().for_each(|term| term.factors.iter().for_each(|factor| {
                    set.insert(&factor.base);
                }));
                rational.denom.iter().for_each(|term| term.factors.iter().for_each(|factor| {
                    set.insert(&factor.base);
                }));
            }
        }
    }

    pub fn substitute(&self, value: &HashMap<&str, usize>) -> usize {
        match self {
            &Self::Constant(value) => value,
            Self::Variable(name) => *value
                .get(&**name)
                .unwrap_or_else(|| panic!("unknown variable \"{name}\"")),
            Self::Sum(operands) => operands.iter().fold(0, |acc, Operand { ty, expr }| {
                let value = expr.substitute(value);
                match ty {
                    Type::Positive => acc + value,
                    Type::Negative => acc.checked_sub(value).unwrap(),
                }
            }),
            Self::Product(operands) => operands.iter().fold(1, |acc, Operand { ty, expr }| {
                let value = expr.substitute(value);
                match ty {
                    Type::Positive => acc * value,
                    Type::Negative => {
                        assert_eq!(acc % value, 0);
                        acc / value
                    }
                }
            }),
            Self::Rational(rational) => {
                // Convert the rational expression result to usize
                let result = rational.substitute(value)
                    .unwrap_or_else(|| panic!("unknown variable in rational expression"));
                // Ensure the result is a whole number
                assert_eq!(result.denom(), &1, "rational expression must evaluate to a whole number");
                result.numer().unsigned_abs()
            }
        }
    }



    /// Checks if two Expr expressions are permanently mathematically equivalent.
    /// Returns:
    /// - `Some(true)` if the expressions are equivalent.
    /// - `Some(false)` if the expressions are not equivalent.
    /// - `None` if the permanent equivalence of the expressions is unknown.
    pub fn equivalent(&self, other: &Self) -> Option<bool> {

        let diff = self.clone() - other.clone();
        let diff_rational = match diff {
            Self::Rational(r) => r,
            _ => RationalExpression::from_dim(&diff)?,
        };
        // Check if there's a non-zero constant term and all other terms are zero
        let mut has_nonzero_constant = false;
        let mut has_nonzero_variable_term = false;
        
        for term in diff_rational.numer.iter() {
            if term.is_constant() {
                if term.coef != Ratio::new(0, 1) {
                    has_nonzero_constant = true;
                }
            } else if term.coef != Ratio::new(0, 1) {
                has_nonzero_variable_term = true;
            }
        }
        if !has_nonzero_constant && !has_nonzero_variable_term {
            Some(true)
        } else if has_nonzero_constant && !has_nonzero_variable_term {
            Some(false)
        } else {
            None
        }
    }

    /// Partially substitute variables with their values.
    /// Returns None if any substituted variable results in a non-integer value.
    pub fn partial_substitute(&self, value: &HashMap<&str, usize>) -> Option<Self> {
        // Convert to rational form first for better handling of complex expressions
        let rational = match self {
            Self::Rational(r) => r.clone(),
            _ => RationalExpression::from_dim(self)?,
        };

        // Perform partial substitution on the rational expression
        let substituted = rational.partial_substitute(value)?;

        // Convert back to Dim
        Some(Self::from(substituted))
    }
}

impl PartialEq for Expr {
    fn eq(&self, other: &Self) -> bool {
        self.equivalent(other) == Some(true)
    }

    #[allow(clippy::partialeq_ne_impl)]
    fn ne(&self, other: &Self) -> bool {
        self.equivalent(other) == Some(false)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Type {
    Positive,
    Negative,
}

impl Type {
    pub fn rev(self) -> Self {
        match self {
            Self::Positive => Self::Negative,
            Self::Negative => Self::Positive,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Operand {
    ty: Type,
    expr: Expr,
}

impl Operand {
    pub fn rev_assign(&mut self) {
        self.ty = self.ty.rev()
    }
}

impl Neg for Operand {
    type Output = Self;

    fn neg(self) -> Self::Output {
        let Self { ty, expr } = self;
        Self { ty: ty.rev(), expr }
    }
}

impl From<usize> for Expr {
    fn from(value: usize) -> Self {
        Expr::Constant(value)
    }
}

impl From<&str> for Expr {
    fn from(value: &str) -> Self {
        Expr::Variable(value.to_string())
    }
}

impl From<String> for Expr {
    fn from(value: String) -> Self {
        Expr::Variable(value)
    }
}

macro_rules! impl_op {
    ($op:ty; $fn:ident; positive: $variant: ident) => {
        impl $op for Expr {
            type Output = Self;
            fn $fn(self, rhs: Self) -> Self::Output {
                match self {
                    Expr::$variant(mut l) => match rhs {
                        Self::$variant(r) => {
                            l.extend(r);
                            Self::$variant(l)
                        }
                        r => {
                            l.push_back(r.positive());
                            Self::$variant(l)
                        }
                    },
                    l => match rhs {
                        Self::$variant(mut r) => {
                            r.push_front(l.positive());
                            Self::$variant(r)
                        }
                        r => Self::$variant([l.positive(), r.positive()].into()),
                    },
                }
            }
        }
    };

    ($op:ty; $fn:ident; negative: $variant: ident) => {
        impl $op for Expr {
            type Output = Self;
            fn $fn(self, rhs: Self) -> Self::Output {
                match self {
                    Expr::$variant(mut l) => match rhs {
                        Self::$variant(r) => {
                            l.extend(r.into_iter().map(Neg::neg));
                            Self::$variant(l)
                        }
                        r => {
                            l.push_back(r.negative());
                            Self::$variant(l)
                        }
                    },
                    l => match rhs {
                        Self::$variant(mut r) => {
                            r.iter_mut().for_each(Operand::rev_assign);
                            r.push_front(l.positive());
                            Self::$variant(r)
                        }
                        r => Self::$variant([l.positive(), r.negative()].into()),
                    },
                }
            }
        }
    };

    ($op:ident; $fn:ident; usize) => {
        impl $op<usize> for Expr {
            type Output = Self;
            fn $fn(self, rhs: usize) -> Self::Output {
                self.$fn(Self::Constant(rhs))
            }
        }
    };
}

impl_op!(Add; add; positive: Sum    );
impl_op!(Sub; sub; negative: Sum    );
impl_op!(Mul; mul; positive: Product);
impl_op!(Div; div; negative: Product);

impl_op!(Add; add; usize);
impl_op!(Sub; sub; usize);
impl_op!(Mul; mul; usize);
impl_op!(Div; div; usize);

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
struct Factor {
    base: String,
    exponent: isize,
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct CanonicalTerm {
    coef: Ratio<isize>,
    factors: Vec<Factor>,  // sorted factors representing the term
}

impl CanonicalTerm {
    fn new(coef: isize) -> Self {
        Self {
            coef: Ratio::new(coef, 1),
            factors: Vec::new(),
        }
    }

    fn neg(&mut self) {
        self.coef = -self.coef;
    }

    fn with_var(coef: isize, var: String) -> Self {
        Self {
            coef: Ratio::new(coef, 1),
            factors: vec![Factor { base: var, exponent: 1 }],
        }
    }

    // Helper function to check if a term is constant
    fn is_constant(&self) -> bool {
        self.factors.is_empty() || self.factors.iter().all(|f| f.exponent == 0)
    }

    fn multiply(&self, other: &Self) -> Self {
        let mut result = Self::new(1);
        result.coef = self.coef * other.coef;
        
        // Combine factors
        let mut factors = self.factors.clone();
        factors.extend(other.factors.iter().cloned());
        
        // Sort and combine like factors
        factors.sort_by(|a, b| a.base.cmp(&b.base));
        let mut combined = Vec::new();
        let mut current: Option<Factor> = None;
        
        for factor in factors {
            match &mut current {
                Some(prev) if prev.base == factor.base => {
                    prev.exponent += factor.exponent;
                }
                _ => {
                    if let Some(prev) = current.take() {
                        if prev.exponent != 0 {
                            combined.push(prev);
                        }
                    }
                    current = Some(factor);
                }
            }
        }
        
        if let Some(prev) = current {
            if prev.exponent != 0 {
                combined.push(prev);
            }
        }
        
        result.factors = combined;
        result
    }

    fn divide(&self, other: &Self) -> Self {
        let mut result = Self::new(1);
        result.coef = self.coef / other.coef;
        
        // Combine factors with negative exponents for division
        let mut factors = self.factors.clone();
        factors.extend(other.factors.iter().map(|f| Factor {
            base: f.base.clone(),
            exponent: -f.exponent,
        }));
        
        // Sort and combine like factors
        factors.sort_by(|a, b| a.base.cmp(&b.base));
        let mut combined = Vec::new();
        let mut current: Option<Factor> = None;
        
        for factor in factors {
            match &mut current {
                Some(prev) if prev.base == factor.base => {
                    prev.exponent += factor.exponent;
                }
                _ => {
                    if let Some(prev) = current.take() {
                        if prev.exponent != 0 {
                            combined.push(prev);
                        }
                    }
                    current = Some(factor);
                }
            }
        }
        
        if let Some(prev) = current {
            if prev.exponent != 0 {
                combined.push(prev);
            }
        }
        
        result.factors = combined;
        result
    }

    /// Combine like terms by adding coefficients of terms with the same variables.
    pub fn combine_like_terms(mut terms: Vec<CanonicalTerm>) -> Vec<CanonicalTerm> {
        // Remove factors with zero exponents from each term
        terms.iter_mut().for_each(|term| {
            term.factors.retain(|factor| factor.exponent != 0);
        });

        // Sort terms so that constant terms come first
        terms.sort_by(|a, b| {
            let a_is_constant = a.is_constant();
            let b_is_constant = b.is_constant();
            
            // If one term is constant and the other isn't, constant comes first
            match (a_is_constant, b_is_constant) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => {
                    // For non-constant terms, sort by their factors
                    let mut a_vars: Vec<_> = a.factors.iter().collect();
                    let mut b_vars: Vec<_> = b.factors.iter().collect();
                    a_vars.sort();
                    b_vars.sort();
                    a_vars.cmp(&b_vars)
                }
            }
        });
        
        let mut result = Vec::new();
        let mut current: Option<CanonicalTerm> = None;
        
        for term in terms {
            match &mut current {
                Some(prev) if (prev.factors == term.factors) || (prev.is_constant() && term.is_constant()) => {
                    prev.coef += term.coef;
                }
                _ => {
                    if let Some(prev) = current.take() {
                        if prev.coef != Ratio::new(0, 1) {
                            result.push(prev);
                        }
                    }
                    current = Some(term);
                }
            }
        }
        
        if let Some(prev) = current {
            if prev.coef != Ratio::new(0, 1) {
                result.push(prev);
            }
        }
        
        result
    }

    fn sum_terms(terms: &[Self], other: &[Self]) -> Vec<Self> {
        let mut result = terms.to_owned();
        result.extend(other.to_owned());
        CanonicalTerm::combine_like_terms(result)
    }

    fn multiply_terms(terms: &Vec<Self>, other: &Vec<Self>) -> Vec<Self> {
        let mut result_terms = Vec::new();
        for term1 in terms {
            for term2 in other {
                result_terms.push(term1.multiply(term2));
            }
        }
        CanonicalTerm::combine_like_terms(result_terms)
    }

    fn terms_divide_by_term(terms: &Vec<Self>, dividend: &Self) -> Vec<Self> {
        let mut result_terms = Vec::new();
        for term in terms {
            result_terms.push(term.divide(dividend));
        }
        CanonicalTerm::combine_like_terms(result_terms)
    }

}

impl PartialOrd for CanonicalTerm {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CanonicalTerm {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.factors.cmp(&other.factors) {
            std::cmp::Ordering::Equal => self.coef.cmp(&other.coef),
            other => other,
        }
    }
}

impl PartialOrd for Factor {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Factor {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.base.cmp(&other.base) {
            std::cmp::Ordering::Equal => self.exponent.cmp(&other.exponent),
            other => other,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RationalExpression {
    numer: Vec<CanonicalTerm>,
    denom: Vec<CanonicalTerm>,
}

impl RationalExpression {
    fn new_zero() -> Self {
        Self { numer: vec![CanonicalTerm::new(0)], denom: vec![CanonicalTerm::new(1)] }
    }

    fn new_one() -> Self {
        Self { numer: vec![CanonicalTerm::new(1)], denom: vec![CanonicalTerm::new(1)] }
    }

    fn new(numer: Vec<CanonicalTerm>, denom: Vec<CanonicalTerm>) -> Self {
        assert!(!denom.is_empty(), "Denominator cannot be empty in RationalExpression");
        Self { numer, denom }
    }

    fn neg(&mut self) {
        self.numer.iter_mut().for_each(CanonicalTerm::neg);
    }

    fn invert(&mut self) {
        std::mem::swap(&mut self.numer, &mut self.denom);
    }

    fn simplify(&self) -> Self {
        // If denominator has only one term, we can simplify by dividing each numerator term
        if self.denom.len() == 1 {
            let denom_term = &self.denom[0];
            let mut simplified_numer = Vec::new();
            
            // Divide each numerator term by the denominator term
            for term in &self.numer {
                simplified_numer.push(term.divide(denom_term));
            }
            
            // Combine like terms in the simplified numerator
            simplified_numer = CanonicalTerm::combine_like_terms(simplified_numer);
            
            // If the simplified numerator is empty, return 0/1
            if simplified_numer.is_empty() {
                return Self::new_zero();
            }
            
            // Return the simplified expression with denominator 1
            Self::new(simplified_numer, vec![CanonicalTerm::new(1)])
        } else {
            // If denominator has multiple terms, return a copy of self
            Self::new(CanonicalTerm::combine_like_terms(self.numer.clone()), CanonicalTerm::combine_like_terms(self.denom.clone()))
        }
    }

    fn from_dim(dim: &Expr) -> Option<Self> {
        match dim {
            Expr::Constant(value) => Some(Self::new(vec![CanonicalTerm::new(*value as isize)], vec![CanonicalTerm::new(1)])),
            Expr::Variable(name) => Some(Self::new(vec![CanonicalTerm::with_var(1, name.clone())], vec![CanonicalTerm::new(1)])),
            Expr::Sum(operands) => {
                let mut result = RationalExpression::new_zero();
                for operand in operands {
                    let sign = match operand.ty {
                        Type::Positive => 1,
                        Type::Negative => -1,
                    };
                    let mut rational = RationalExpression::from_dim(&operand.expr)?;
                    if sign == -1 {
                        rational.neg();
                    }
                    result = RationalExpression::new(
                        CanonicalTerm::sum_terms(&CanonicalTerm::multiply_terms(&result.numer, &rational.denom), &CanonicalTerm::multiply_terms(&rational.numer, &result.denom)),
                        CanonicalTerm::multiply_terms(&result.denom, &rational.denom),
                    );
                }
                Some(result)
            }
            Expr::Product(operands) => {
                let mut result = RationalExpression::new_one();

                for operand in operands {
                    let sign = match operand.ty {
                        Type::Positive => 1,
                        Type::Negative => -1,
                    };
                    let mut rational = RationalExpression::from_dim(&operand.expr)?;
                    if sign == -1 {
                        rational.invert();
                    }

                    if rational.denom.len() > 1 {
                        result = RationalExpression::new(
                            CanonicalTerm::multiply_terms(&result.numer, &rational.numer),
                            CanonicalTerm::multiply_terms(&result.denom, &rational.denom),
                        );
                    } else {
                        result.numer = CanonicalTerm::multiply_terms(&result.numer, &rational.numer);
                        result.numer = CanonicalTerm::terms_divide_by_term(&result.numer, &rational.denom[0]);
                    }
                }
                Some(result)
            }
            Expr::Rational(rational) => Some(rational.clone()),
        }
    }

    /// Substitute variables with their values in the rational expression.
    /// Returns None if any variable in the expression is not found in the substitution map.
    pub fn substitute(&self, value: &HashMap<&str, usize>) -> Option<Ratio<isize>> {
        // Helper function to substitute a single term
        fn substitute_term(term: &CanonicalTerm, value: &HashMap<&str, usize>) -> Option<Ratio<isize>> {
            let mut result = term.coef;
            for factor in &term.factors {
                let var_value = value.get(factor.base.as_str())?;
                if factor.exponent > 0 {
                    for _ in 0..factor.exponent {
                        result *= Ratio::from_integer(*var_value as isize);
                    }
                } else {
                    for _ in 0..-factor.exponent {
                        result /= Ratio::from_integer(*var_value as isize);
                    }
                }
            }
            Some(result)
        }

        // Substitute numerator terms
        let mut numer_value = Ratio::from_integer(0);
        for term in &self.numer {
            numer_value += substitute_term(term, value)?;
        }

        // Substitute denominator terms
        let mut denom_value = Ratio::from_integer(0);
        for term in &self.denom {
            denom_value += substitute_term(term, value)?;
        }

        // Return the result of division
        Some(numer_value / denom_value)
    }

    /// Partially substitute variables with their values.
    /// Returns None if any substituted variable results in a non-integer value.
    pub fn partial_substitute(&self, value: &HashMap<&str, usize>) -> Option<Self> {
        // Helper function to substitute a single term
        fn substitute_term(term: &CanonicalTerm, value: &HashMap<&str, usize>) -> Option<CanonicalTerm> {
            let mut result = term.clone();
            for factor in &mut result.factors {
                if let Some(&var_value) = value.get(factor.base.as_str()) {
                    if factor.exponent > 0 {
                        for _ in 0..factor.exponent {
                            result.coef *= Ratio::from_integer(var_value as isize);
                        }
                    } else {
                        for _ in 0..-factor.exponent {
                            result.coef /= Ratio::from_integer(var_value as isize);
                        }
                    }
                    // Remove the substituted factor
                    factor.exponent = 0;
                }
            }
            // Remove factors with zero exponents
            result.factors.retain(|f| f.exponent != 0);
            Some(result)
        }

        let mut new_numer = Vec::new();
        for term in &self.numer {
            new_numer.push(substitute_term(term, value)?);
        }

        let mut new_denom = Vec::new();
        for term in &self.denom {
            new_denom.push(substitute_term(term, value)?);
        }

        // Combine like terms and simplify
        let result = Self::new(
            CanonicalTerm::combine_like_terms(new_numer),
            CanonicalTerm::combine_like_terms(new_denom)
        ).simplify();

        Some(result)
    }
}

impl PartialOrd for RationalExpression {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RationalExpression {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.numer.cmp(&other.numer) {
            std::cmp::Ordering::Equal => self.denom.cmp(&other.denom),
            other => other,
        }
    }
}

impl From<RationalExpression> for Expr {
    fn from(rational: RationalExpression) -> Self {
        Expr::Rational(rational.simplify())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dim_example() {
        let a = Expr::var("a");
        let b = Expr::var("b");
        let _1 = Expr::from(1);
        let expr = (a + _1 - 2) * 3 / (b + 1);
        assert_eq!(expr.substitute(&HashMap::from([("a", 8), ("b", 6)])), 3);
    }

    #[test]
    fn test_dim_equivalence() {
        // Test constant equivalence
        assert_eq!(Expr::from(1).equivalent(&Expr::from(1)), Some(true));
        assert_eq!(Expr::from(1).equivalent(&Expr::from(2)), Some(false));

        // Test variable equivalence
        let a = Expr::var("a");
        let b = Expr::var("b");
        println!("asserting a != b");
        assert_eq!(a.equivalent(&b), None);

        // Test sum equivalence
        let expr1 = a.clone() + 1;
        let expr2 = a.clone() + 1;
        let expr3 = a.clone() + 2;
        println!("asserting a + 1 == a + 1");
        assert!(expr1 == expr2);
        assert_eq!(expr1.equivalent(&expr2), Some(true));
        println!("asserting a + 1 != a + 2");
        assert!(expr1 != expr3);
        assert_eq!(expr1.equivalent(&expr3), Some(false));

        // Test product equivalence
        let expr4 = a.clone() * 2;
        let expr5 = a.clone() * 2;
        let expr6 = a.clone() * 3;
        println!("asserting a * 2 == a * 2");
        assert!(expr4 == expr5);
        assert_eq!(expr4.equivalent(&expr5), Some(true));
        println!("asserting a * 2 != a * 3");
        assert!(!(expr4 == expr6));
        assert!(!(expr4 != expr6));
        assert_eq!(expr4.equivalent(&expr6), None);

        // Test complex expression equivalence
        let complex1 = (a.clone() + 1) * 2;
        let complex2 = a.clone() * 2 + 2;
        let complex3 = a.clone() * 2 + 3;
        println!("asserting (a + 1) * 2 == a * 2 + 2");
        assert!(complex1 == complex2);
        assert_eq!(complex1.equivalent(&complex2), Some(true)); // (a + 1) * 2 = a * 2 + 2
        println!("asserting (a + 1) * 2 != a * 2 + 3");
        assert!(complex1 != complex3);
        assert_eq!(complex1.equivalent(&complex3), Some(false));

        // Test commutative operations
        let expr7 = a.clone() + b.clone();
        let expr8 = b.clone() + a.clone();
        println!("asserting a + b == b + a");
        assert!(expr7 == expr8);
        assert_eq!(expr7.equivalent(&expr8), Some(true)); // a + b = b + a

        let expr9 = a.clone() * b.clone();
        let expr10 = b.clone() * a.clone();
        println!("asserting a * b == b * a");
        assert!(expr9 == expr10);
        assert_eq!(expr9.equivalent(&expr10), Some(true)); // a * b = b * a

        // Test distributive property
        let expr11 = a.clone() * (b.clone() + 1);
        let expr12 = a.clone() * b.clone() + a.clone();
        println!("asserting a * (b + 1) == a * b + a");
        assert!(expr11 == expr12);
        assert_eq!(expr11.equivalent(&expr12), Some(true)); // a * (b + 1) = a * b + a

        // Test division and complex expressions
        let c = Expr::var("c");

        // Test division equivalence
        let expr13 = (a.clone() * b.clone()) / c.clone();
        let expr14 = a.clone() * (b.clone() / c.clone());
        println!("asserting (a * b) / c == a * (b / c)");
        assert!(expr13 == expr14);
        assert_eq!(expr13.equivalent(&expr14), Some(true)); // (a * b) / c = a * (b / c)

        // Test mixed operations with division
        let expr15 = (a.clone() + b.clone()) / c.clone();
        let expr16 = a.clone() / c.clone() + b.clone() / c.clone();
        println!("asserting (a + b) / c == a/c + b/c");
        assert!(expr15 == expr16);
        assert_eq!(expr15.equivalent(&expr16), Some(true)); // (a + b) / c = a/c + b/c

        // Test complex nested expressions
        let expr17 = (a.clone() * b.clone() + c.clone()) / (a.clone() + Expr::from(1));
        let expr18 = (b.clone() * a.clone() + c.clone()) / (Expr::from(1) + a.clone());
        println!("asserting (a*b + c)/(a + 1) == (b*a + c)/(1 + a)");
        assert!(expr17 == expr18);
        assert_eq!(expr17.equivalent(&expr18), Some(true)); // (a*b + c)/(a + 1) = (b*a + c)/(1 + a)

        // Test expressions with multiple divisions
        let expr19 = (a.clone() / b.clone()) / c.clone();
        let expr20 = a.clone() / (b.clone() * c.clone());
        println!("asserting (a/b)/c == a/(b*c)");
        assert!(expr19 == expr20);
        assert_eq!(expr19.equivalent(&expr20), Some(true)); // (a/b)/c = a/(b*c)

        // Test expressions with constants and variables
        let expr21 = (a.clone() * Expr::from(2) + b.clone() * Expr::from(3)) / Expr::from(6);
        let expr22 = a.clone() / Expr::from(3) + b.clone() / Expr::from(2);
        println!("asserting (2a + 3b)/6 == a/3 + b/2");
        assert!(expr21 == expr22);
        assert_eq!(expr21.equivalent(&expr22), Some(true)); // (2a + 3b)/6 = a/3 + b/2

        // Test expressions with nested divisions and multiplications
        let expr23 = a.clone() * (b.clone() / (c.clone() * Expr::from(2)));
        let expr24 = (a.clone() * b.clone()) / (Expr::from(2) * c.clone());
        println!("asserting a * (b/(c*2)) == (a*b)/(2*c)");
        assert!(expr23 == expr24);
        assert_eq!(expr23.equivalent(&expr24), Some(true)); // a * (b/(c*2)) = (a*b)/(2*c)

        // Test expressions with subtraction and division
        let expr25 = (a.clone() - b.clone()) / c.clone();
        let expr26 = a.clone() / c.clone() - b.clone() / c.clone();
        println!("asserting (a - b)/c == a/c - b/c");
        assert!(expr25 == expr26);
        assert_eq!(expr25.equivalent(&expr26), Some(true)); // (a - b)/c = a/c - b/c

        // Test expressions with multiple operations
        let expr27 = (a.clone() * b.clone() + c.clone() * Expr::from(2)) / (b.clone() + Expr::from(2));
        let expr28 = (b.clone() * a.clone() + Expr::from(2) * c.clone()) / (Expr::from(2) + b.clone());
        println!("asserting (a*b + 2c)/(b + 2) == (b*a + 2c)/(2 + b)");
        assert!(expr27 == expr28);
        assert_eq!(expr27.equivalent(&expr28), Some(true)); // (a*b + 2c)/(b + 2) = (b*a + 2c)/(2 + b)
    }

    #[test]
    fn test_power_expressions() {
        let a = Expr::var("a");
        let b = Expr::var("b");
        let c = Expr::var("c");

        // Test simple power expressions
        let pow1 = a.clone() * a.clone();
        let pow2 = a.clone() * a.clone();
        println!("asserting a * a == a * a");
        assert!(pow1 == pow2);
        assert_eq!(pow1.equivalent(&pow2), Some(true));

        // Test power with division
        let pow3 = (a.clone() * a.clone()) / b.clone();
        let pow4 = a.clone() * (a.clone() / b.clone());
        println!("asserting (a * a) / b == a * (a / b)");
        assert!(pow3 == pow4);
        assert_eq!(pow3.equivalent(&pow4), Some(true));

        // Test multiple variables with powers
        let pow5 = (a.clone() * a.clone() * b.clone()) / (c.clone() * c.clone());
        let pow6 = (a.clone() * b.clone()) * (a.clone() / (c.clone() * c.clone()));
        println!("asserting (a² * b) / c² == (a * b) * (a / c²)");
        assert!(pow5 == pow6);
        assert_eq!(pow5.equivalent(&pow6), Some(true));

        // Test complex power expressions with constants
        let pow7 = (a.clone() * a.clone() * Expr::from(2) + b.clone() * b.clone() * Expr::from(3)) / Expr::from(6);
        let pow8 = (a.clone() * a.clone()) / Expr::from(3) + (b.clone() * b.clone()) / Expr::from(2);
        println!("asserting (2a² + 3b²)/6 == a²/3 + b²/2");
        assert!(pow7 == pow8);
        assert_eq!(pow7.equivalent(&pow8), Some(true));

        // Test nested power expressions
        let pow9 = (a.clone() * a.clone() + b.clone()) / (a.clone() * c.clone());
        let pow10 = a.clone() / c.clone() + b.clone() / (a.clone() * c.clone());
        println!("asserting (a² + b)/(a * c) == a/c + b/(a * c)");
        assert!(pow9 == pow10);
        assert_eq!(pow9.equivalent(&pow10), Some(true));

        // Test power expressions with multiple operations
        let pow11 = (a.clone() * a.clone() * b.clone() + c.clone() * c.clone()) / (b.clone() + Expr::from(2));
        let pow12 = (b.clone() * a.clone() * a.clone() + c.clone() * c.clone()) / (Expr::from(2) + b.clone());
        println!("asserting (a² * b + c²)/(b + 2) == (b * a² + c²)/(2 + b)");
        assert!(pow11 == pow12);
        assert_eq!(pow11.equivalent(&pow12), Some(true));

        // Test power expressions with division and multiplication
        let pow13 = (a.clone() * a.clone()) / (b.clone() * b.clone()) * c.clone();
        let pow14 = (a.clone() * a.clone() * c.clone()) / (b.clone() * b.clone());
        println!("asserting (a²/b²) * c == (a² * c)/b²");
        assert!(pow13 == pow14);
        assert_eq!(pow13.equivalent(&pow14), Some(true));

        // Test sum of terms with same variable but different exponents
        let sum1 = a.clone() + a.clone() * a.clone() + a.clone() * a.clone() * a.clone();
        let sum2 = a.clone() * a.clone() * a.clone() + a.clone() * a.clone() + a.clone();
        println!("asserting a + a² + a³ == a³ + a² + a");
        assert!(sum1 == sum2);
        assert_eq!(sum1.equivalent(&sum2), Some(true));

        // Test sum of terms with same variable and coefficients
        let sum3 = a.clone() * Expr::from(2) + a.clone() * a.clone() * Expr::from(3) + a.clone() * a.clone() * a.clone() * Expr::from(4);
        let sum4 = a.clone() * a.clone() * a.clone() * Expr::from(4) + a.clone() * Expr::from(2) + a.clone() * a.clone() * Expr::from(3);
        println!("asserting 2a + 3a² + 4a³ == 4a³ + 2a + 3a²");
        assert!(sum3 == sum4);
        assert_eq!(sum3.equivalent(&sum4), Some(true));

        // Test sum of terms with same variable and negative coefficients
        let sum5 = a.clone() * Expr::from(2) - a.clone() * a.clone() * Expr::from(3) + a.clone() * a.clone() * a.clone() * Expr::from(4);
        let sum6 = a.clone() * a.clone() * a.clone() * Expr::from(4) + a.clone() * Expr::from(2) - a.clone() * a.clone() * Expr::from(3);
        println!("asserting 2a - 3a² + 4a³ == 4a³ + 2a - 3a²");
        assert!(sum5 == sum6);
        assert_eq!(sum5.equivalent(&sum6), Some(true));

        // Test sum of terms with same variable and mixed operations
        let sum7 = (a.clone() * a.clone() + a.clone()) / Expr::from(2) + a.clone() * a.clone() * a.clone();
        let sum8 = a.clone() * a.clone() * a.clone() + (a.clone() + a.clone() * a.clone()) / Expr::from(2);
        println!("asserting (a² + a)/2 + a³ == a³ + (a + a²)/2");
        assert!(sum7 == sum8);
        assert_eq!(sum7.equivalent(&sum8), Some(true));
    }

    #[test]
    fn test_complex_expressions() {
        let a = Expr::var("a");
        let b = Expr::var("b");
        let c = Expr::var("c");
        
        // Test complex denominator expressions
        let complex1 = (a.clone() * a.clone() * b.clone()) / ((c.clone() + Expr::from(1)) * (c.clone() + Expr::from(2)));
        let complex2 = (a.clone() * a.clone() * b.clone()) / (c.clone() * c.clone() + c.clone() * 3 + Expr::from(2));
        println!("asserting (a² * b)/((c+1)(c+2)) == (a² * b)/(c² + 3c + 2)");
        assert!(complex1 == complex2);
        assert_eq!(complex1.equivalent(&complex2), Some(true));
    }

    #[test]
    fn test_factor_ordering() {
        let a = Expr::var("a");
        let b = Expr::var("b");
        
        // Test ordering of terms with same base but different exponents
        let expr1 = a.clone() + a.clone() * a.clone() + a.clone() * a.clone() * a.clone();
        let expr2 = a.clone() * a.clone() * a.clone() + a.clone() * a.clone() + a.clone();
        println!("asserting a + a² + a³ == a³ + a² + a");
        assert!(expr1 == expr2);
        assert_eq!(expr1.equivalent(&expr2), Some(true));

        // Test ordering of terms with different bases and exponents
        let expr3 = a.clone() + b.clone() + a.clone() * a.clone() + b.clone() * b.clone();
        let expr4 = b.clone() * b.clone() + a.clone() * a.clone() + b.clone() + a.clone();
        println!("asserting a + b + a² + b² == b² + a² + b + a");
        assert!(expr3 == expr4);
        assert_eq!(expr3.equivalent(&expr4), Some(true));

        // Test ordering with negative exponents
        let expr5 = a.clone() / b.clone() + a.clone() * a.clone() / b.clone();
        let expr6 = a.clone() * a.clone() / b.clone() + a.clone() / b.clone();
        println!("asserting a/b + a²/b == a²/b + a/b");
        assert!(expr5 == expr6);
        assert_eq!(expr5.equivalent(&expr6), Some(true));
    }

    #[test]
    fn test_rational_expression_into_dim() {
        let a = Expr::var("a");
        let b = Expr::var("b");
        let c = Expr::var("c");

        // Test simple constant expressions
        let rational1 = RationalExpression::new(
            vec![CanonicalTerm::new(5)],
            vec![CanonicalTerm::new(1)]
        );
        let dim1: Expr = rational1.into();
        assert!(dim1 == Expr::Constant(5));

        // Test simple variable expressions
        let rational2 = RationalExpression::new(
            vec![CanonicalTerm::with_var(1, "a".to_string())],
            vec![CanonicalTerm::new(1)]
        );
        let dim2: Expr = rational2.into();
        assert!(dim2 == a.clone());

        // Test expressions with negative coefficients
        let rational3 = RationalExpression::new(
            vec![CanonicalTerm::new(-3)],
            vec![CanonicalTerm::new(1)]
        );
        let dim3: Expr = rational3.into();
        let expected3 = Expr::Sum(VecDeque::from([Operand { ty: Type::Negative, expr: Expr::Constant(3) }]));
        assert!(dim3 == expected3);

        // Test simple division
        let rational4 = RationalExpression::new(
            vec![CanonicalTerm::with_var(1, "a".to_string())],
            vec![CanonicalTerm::with_var(1, "b".to_string())]
        );
        let dim4: Expr = rational4.into();
        assert!(dim4 == a.clone() / b.clone());

        // Test complex rational expressions
        let rational5 = RationalExpression::new(
            vec![
                CanonicalTerm::with_var(2, "a".to_string()),
                CanonicalTerm::with_var(3, "b".to_string())
            ],
            vec![CanonicalTerm::new(6)]
        );
        let dim5: Expr = rational5.into();
        assert!(dim5 == (a.clone() * 2 + b.clone() * 3) / 6);

        // Test expressions with multiple variables and exponents
        let rational6 = RationalExpression::new(
            vec![
                CanonicalTerm {
                    coef: Ratio::new(1, 1),
                    factors: vec![
                        Factor { base: "a".to_string(), exponent: 2 },
                        Factor { base: "b".to_string(), exponent: 1 }
                    ]
                }
            ],
            vec![
                CanonicalTerm {
                    coef: Ratio::new(1, 1),
                    factors: vec![
                        Factor { base: "c".to_string(), exponent: 2 }
                    ]
                }
            ]
        );
        let dim6: Expr = rational6.into();
        assert!(dim6 == (a.clone() * a.clone() * b.clone()) / (c.clone() * c.clone()));

        // Test expressions with negative exponents
        let rational7 = RationalExpression::new(
            vec![
                CanonicalTerm {
                    coef: Ratio::new(1, 1),
                    factors: vec![
                        Factor { base: "a".to_string(), exponent: 1 },
                        Factor { base: "b".to_string(), exponent: -1 }
                    ]
                }
            ],
            vec![CanonicalTerm::new(1)]
        );
        let dim7: Expr = rational7.into();
        assert!(dim7 == a.clone() / b.clone());

        // Test expressions with multiple terms in numerator and denominator
        let rational8 = RationalExpression::new(
            vec![
                CanonicalTerm::with_var(2, "a".to_string()),
                CanonicalTerm::with_var(3, "b".to_string())
            ],
            vec![
                CanonicalTerm::with_var(1, "c".to_string()),
                CanonicalTerm::new(2)
            ]
        );
        let dim8: Expr = rational8.into();
        assert!(dim8 == (a.clone() * 2 + b.clone() * 3) / (c.clone() + 2));

        // Test empty numerator (should become 0)
        let rational9 = RationalExpression::new(
            vec![],
            vec![CanonicalTerm::new(1)]
        );
        let dim9: Expr = rational9.into();
        assert!(dim9 == Expr::Constant(0));
    }

    #[test]
    #[should_panic(expected = "Denominator cannot be empty in RationalExpression")]
    fn test_rational_expression_empty_denominator() {
        let rational = RationalExpression::new(
            vec![CanonicalTerm::new(5)],
            vec![]
        );
        let _: Expr = rational.into();
    }

    #[test]
    fn test_rational_function_equivalence() {
        let a = Expr::var("a");
        let b = Expr::var("b");
        let c = Expr::var("c");
        let d = Expr::var("d");

        // Test with common factor
        let rational1 = (a.clone() + b.clone()) / (c.clone() + d.clone());
        let common_factor = a.clone() + b.clone() + c.clone();
        let rational2 = ((a.clone() + b.clone()) * common_factor.clone()) / ((c.clone() + d.clone()) * common_factor);
        println!("asserting (a + b)/(c + d) == ((a + b)(a + b + c))/((c + d)(a + b + c))");
        assert!(rational1 == rational2);
        assert_eq!(rational1.equivalent(&rational2), Some(true));

        // Test with common constant
        let coef1 = (a.clone() + b.clone()) / (c.clone() + d.clone());
        let coef2 = ((a.clone() + b.clone()) * 2) / ((c.clone() + d.clone()) * 2);
        println!("asserting (a + b)/(c + d) == (2(a + b))/(2(c + d))");
        assert!(coef1 == coef2);
        assert_eq!(coef1.equivalent(&coef2), Some(true));

        // Test with different common constants
        let coef3 = ((a.clone() + b.clone()) * 2) / ((c.clone() + d.clone()) * 2);
        let coef4 = ((a.clone() + b.clone()) * 4) / ((c.clone() + d.clone()) * 4);
        println!("asserting (2(a + b))/(2(c + d)) == (4(a + b))/(4(c + d))");
        assert!(coef3 == coef4);
        assert_eq!(coef3.equivalent(&coef4), Some(true));

        // Test with more complex expressions and common factor
        let complex1 = (a.clone() * b.clone() + c.clone()) / (a.clone() + d.clone());
        let common_factor2 = a.clone() * b.clone() + c.clone() + d.clone();
        let complex2 = ((a.clone() * b.clone() + c.clone()) * common_factor2.clone()) / ((a.clone() + d.clone()) * common_factor2);
        println!("asserting (ab + c)/(a + d) == ((ab + c)(ab + c + d))/((a + d)(ab + c + d))");
        assert!(complex1 == complex2);
        assert_eq!(complex1.equivalent(&complex2), Some(true));

        // Test with more complex expressions and common constant
        let coef5 = (a.clone() * b.clone() + c.clone()) / (a.clone() + d.clone());
        let coef6 = ((a.clone() * b.clone() + c.clone()) * 3) / ((a.clone() + d.clone()) * 3);
        println!("asserting (ab + c)/(a + d) == (3(ab + c))/(3(a + d))");
        assert!(coef5 == coef6);
        assert_eq!(coef5.equivalent(&coef6), Some(true));

        // Test with different common constants in complex expressions
        let coef7 = ((a.clone() * b.clone() + c.clone()) * 3) / ((a.clone() + d.clone()) * 3);
        let coef8 = ((a.clone() * b.clone() + c.clone()) * 6) / ((a.clone() + d.clone()) * 6);
        println!("asserting (3(ab + c))/(3(a + d)) == (6(ab + c))/(6(a + d))");
        assert!(coef7 == coef8);
        assert_eq!(coef7.equivalent(&coef8), Some(true));

        // Test with expressions containing constants and common factor
        let const1 = (a.clone() * 2 + b.clone() * 3) / (c.clone() + 4);
        let common_factor3 = a.clone() + b.clone() + c.clone();
        let const2 = ((a.clone() * 2 + b.clone() * 3) * common_factor3.clone()) / ((c.clone() + 4) * common_factor3);
        println!("asserting (2a + 3b)/(c + 4) == ((2a + 3b)(a + b + c))/((c + 4)(a + b + c))");
        assert!(const1 == const2);
        assert_eq!(const1.equivalent(&const2), Some(true));

        // Test with expressions containing constants and common constant
        let coef9 = (a.clone() * 2 + b.clone() * 3) / (c.clone() + 4);
        let coef10 = ((a.clone() * 2 + b.clone() * 3) * 5) / ((c.clone() + 4) * 5);
        println!("asserting (2a + 3b)/(c + 4) == (5(2a + 3b))/(5(c + 4))");
        assert!(coef9 == coef10);
        assert_eq!(coef9.equivalent(&coef10), Some(true));

        // Test with different common constants in expressions with constants
        let coef11 = ((a.clone() * 2 + b.clone() * 3) * 5) / ((c.clone() + 4) * 5);
        let coef12 = ((a.clone() * 2 + b.clone() * 3) * 10) / ((c.clone() + 4) * 10);
        println!("asserting (5(2a + 3b))/(5(c + 4)) == (10(2a + 3b))/(10(c + 4))");
        assert!(coef11 == coef12);
        assert_eq!(coef11.equivalent(&coef12), Some(true));

        // Test with nested expressions and common factor
        let nested1 = ((a.clone() + b.clone()) * c.clone()) / ((a.clone() - b.clone()) * d.clone());
        let common_factor4 = a.clone() * b.clone() + c.clone() * d.clone();
        let nested2 = (((a.clone() + b.clone()) * c.clone()) * common_factor4.clone()) / (((a.clone() - b.clone()) * d.clone()) * common_factor4);
        println!("asserting ((a + b)c)/((a - b)d) == ((a + b)c(ab + cd))/((a - b)d(ab + cd))");
        assert!(nested1 == nested2);
        assert_eq!(nested1.equivalent(&nested2), Some(true));

        // Test with nested expressions and common constant
        let coef13 = ((a.clone() + b.clone()) * c.clone()) / ((a.clone() - b.clone()) * d.clone());
        let coef14 = (((a.clone() + b.clone()) * c.clone()) * 4) / (((a.clone() - b.clone()) * d.clone()) * 4);
        println!("asserting ((a + b)c)/((a - b)d) == (4((a + b)c))/(4((a - b)d))");
        assert!(coef13 == coef14);
        assert_eq!(coef13.equivalent(&coef14), Some(true));

        // Test with different common constants in nested expressions
        let coef15 = (((a.clone() + b.clone()) * c.clone()) * 4) / (((a.clone() - b.clone()) * d.clone()) * 4);
        let coef16 = (((a.clone() + b.clone()) * c.clone()) * 8) / (((a.clone() - b.clone()) * d.clone()) * 8);
        println!("asserting (4((a + b)c))/(4((a - b)d)) == (8((a + b)c))/(8((a - b)d))");
        assert!(coef15 == coef16);
        assert_eq!(coef15.equivalent(&coef16), Some(true));
    }

    #[test]
    fn test_substitute() {
        let a = Expr::var("a");
        let b = Expr::var("b");
        let c = Expr::var("c");

        // Test simple variable substitution
        let expr1 = a.clone() + b.clone();
        let values1 = HashMap::from([("a", 2), ("b", 3)]);
        println!("asserting a + b == 5");
        assert_eq!(expr1.substitute(&values1), 5);

        // Test multiplication and division
        let expr2 = (a.clone() * b.clone()) / c.clone();
        let values2 = HashMap::from([("a", 6), ("b", 4), ("c", 3)]);
        println!("asserting (ab)/c == 8");
        assert_eq!(expr2.substitute(&values2), 8);

        // Test complex expressions
        let expr3 = (a.clone() * 2 + b.clone() * 3) / (c.clone() + 4);
        let values3 = HashMap::from([("a", 5), ("b", 10), ("c", 6)]);
        println!("asserting (2a + 3b)/(c + 4) == 4");
        assert_eq!(expr3.substitute(&values3), 4);

        // Test rational expressions with single-term denominator
        let rational1 = RationalExpression::new(
            vec![
                CanonicalTerm::with_var(2, "a".to_string()),
                CanonicalTerm::with_var(3, "b".to_string())
            ],
            vec![CanonicalTerm::new(6)]
        );
        let values4 = HashMap::from([("a", 6), ("b", 4)]);
        println!("asserting (2a + 3b)/6 == 4");
        assert_eq!(rational1.substitute(&values4), Some(Ratio::new(4, 1)));

        // Test rational expressions with exponents
        let rational3 = RationalExpression::new(
            vec![
                CanonicalTerm {
                    coef: Ratio::new(1, 1),
                    factors: vec![
                        Factor { base: "a".to_string(), exponent: 2 },
                        Factor { base: "b".to_string(), exponent: 1 }
                    ]
                }
            ],
            vec![
                CanonicalTerm {
                    coef: Ratio::new(1, 1),
                    factors: vec![
                        Factor { base: "c".to_string(), exponent: 2 }
                    ]
                }
            ]
        );
        let values6 = HashMap::from([("a", 4), ("b", 2), ("c", 2)]);
        println!("asserting (a²b)/c² == 8");
        assert_eq!(rational3.substitute(&values6), Some(Ratio::new(8, 1)));

        // Test with negative exponents
        let rational5 = RationalExpression::new(
            vec![
                CanonicalTerm {
                    coef: Ratio::new(1, 1),
                    factors: vec![
                        Factor { base: "a".to_string(), exponent: 1 },
                        Factor { base: "b".to_string(), exponent: -1 }
                    ]
                }
            ],
            vec![CanonicalTerm::new(1)]
        );
        let values9 = HashMap::from([("a", 6), ("b", 2)]);
        println!("asserting a/b == 3");
        assert_eq!(rational5.substitute(&values9), Some(Ratio::new(3, 1)));
    }

    #[test]
    #[should_panic(expected = "unknown variable")]
    fn test_substitute_unknown_variable() {
        let a = Expr::var("a");
        let values = HashMap::from([("b", 2)]);
        let _ = a.substitute(&values);
    }

    #[test]
    #[should_panic(expected = "rational expression must evaluate to a whole number")]
    fn test_substitute_non_integer_result() {
        let rational = RationalExpression::new(
            vec![CanonicalTerm::new(1)],
            vec![CanonicalTerm::new(2)]
        );
        let values = HashMap::new();
        let dim = Expr::Rational(rational);
        let _ = dim.substitute(&values);
    }

    #[test]
    fn test_variables() {
        let a = Expr::var("a");
        let b = Expr::var("b");
        let c = Expr::var("c");
        let d = Expr::var("d");

        // Test simple variable
        assert_eq!(a.variables(), BTreeSet::from(["a"]));

        // Test constant
        assert_eq!(Expr::from(1).variables(), BTreeSet::<&str>::new());

        // Test sum
        let sum = a.clone() + b.clone() + c.clone();
        assert_eq!(sum.variables(), BTreeSet::from(["a", "b", "c"]));

        // Test product
        let prod = a.clone() * b.clone() * c.clone();
        assert_eq!(prod.variables(), BTreeSet::from(["a", "b", "c"]));

        // Test complex expression
        let complex = (a.clone() * b.clone() + c.clone()) / (d.clone() + 1);
        assert_eq!(complex.variables(), BTreeSet::from(["a", "b", "c", "d"]));

        // Test rational expression
        let rational = RationalExpression::new(
            vec![
                CanonicalTerm::with_var(1, "a".to_string()),
                CanonicalTerm::with_var(1, "b".to_string())
            ],
            vec![
                CanonicalTerm::with_var(1, "c".to_string()),
                CanonicalTerm::new(1)
            ]
        );
        let dim_rational = Expr::Rational(rational);
        assert_eq!(dim_rational.variables(), BTreeSet::from(["a", "b", "c"]));
    }

    #[test]
    fn test_partial_substitute() {
        let a = Expr::var("a");
        let b = Expr::var("b");
        let c = Expr::var("c");
        let d = Expr::var("d");

        // Test simple partial substitution
        let expr1 = a.clone() + b.clone();
        let values1 = HashMap::from([("a", 2)]);
        let result1 = expr1.partial_substitute(&values1).unwrap();
        println!("asserting a + b == 2 + b (a = 2)");
        assert_eq!(result1, Expr::from(2) + b.clone());

        // Test multiplication and division
        let expr2 = (a.clone() * b.clone()) / c.clone();
        let values2 = HashMap::from([("a", 6), ("c", 3)]);
        let result2 = expr2.partial_substitute(&values2).unwrap();
        println!("asserting (ab)/c == 2b (a = 6, c = 3)");
        assert_eq!(result2, (Expr::from(6) * b.clone()) / Expr::from(3));

        // Test complex expressions
        let expr3 = (a.clone() * 2 + b.clone() * 3) / (c.clone() + 4);
        let values3 = HashMap::from([("a", 5), ("c", 6)]);
        let result3 = expr3.partial_substitute(&values3).unwrap();
        println!("asserting (2a + 3b)/(c + 4) == (10 + 3b)/(6 + 4) (a = 5, c = 6)");
        assert_eq!(result3, (Expr::from(10) + b.clone() * 3) / (Expr::from(6) + 4));

        // Test rational expressions
        let rational = RationalExpression::new(
            vec![
                CanonicalTerm::with_var(2, "a".to_string()),
                CanonicalTerm::with_var(3, "b".to_string())
            ],
            vec![CanonicalTerm::new(6)]
        );
        let values4 = HashMap::from([("a", 6)]);
        let result4 = rational.partial_substitute(&values4).unwrap();
        println!("asserting (2a + 3b)/6 == 12 + 3b (a = 6)");
        assert_eq!(result4, RationalExpression::new(
            vec![
                CanonicalTerm::new(12),
                CanonicalTerm::with_var(3, "b".to_string())
            ],
            vec![CanonicalTerm::new(6)]
        ).simplify());

        // Test with exponents
        let rational2 = RationalExpression::new(
            vec![
                CanonicalTerm {
                    coef: Ratio::new(1, 1),
                    factors: vec![
                        Factor { base: "a".to_string(), exponent: 2 },
                        Factor { base: "b".to_string(), exponent: 1 }
                    ]
                }
            ],
            vec![
                CanonicalTerm {
                    coef: Ratio::new(1, 1),
                    factors: vec![
                        Factor { base: "c".to_string(), exponent: 2 }
                    ]
                }
            ]
        );
        let values5 = HashMap::from([("a", 4), ("c", 2)]);
        let result5 = rational2.partial_substitute(&values5).unwrap();
        println!("asserting (a²b)/c² == 16b/4 (a = 4, c = 2)");
        assert_eq!(result5, RationalExpression::new(
            vec![
                CanonicalTerm {
                    coef: Ratio::new(16, 1),
                    factors: vec![
                        Factor { base: "b".to_string(), exponent: 1 }
                    ]
                }
            ],
            vec![
                CanonicalTerm {
                    coef: Ratio::new(4, 1),
                    factors: vec![]
                }
            ]
        ).simplify());

        // Test complex nested expressions
        let expr4 = ((a.clone() + b.clone()) * c.clone()) / ((a.clone() - b.clone()) * d.clone());
        let values6 = HashMap::from([("a", 4), ("c", 2)]);
        let result6 = expr4.partial_substitute(&values6).unwrap();
        println!("asserting ((a + b)c)/((a - b)d) == ((4 + b)2)/((4 - b)d) (a = 4, c = 2)");
        assert_eq!(result6, ((Expr::from(4) + b.clone()) * Expr::from(2)) / ((Expr::from(4) - b.clone()) * d.clone()));

        // Test expressions with multiple operations
        let expr5 = (a.clone() * b.clone() + c.clone() * Expr::from(2)) / (b.clone() + Expr::from(2));
        let values7 = HashMap::from([("a", 3), ("c", 4)]);
        let result7 = expr5.partial_substitute(&values7).unwrap();
        println!("asserting (ab + 2c)/(b + 2) == (3b + 8)/(b + 2) (a = 3, c = 4)");
        assert_eq!(result7, (Expr::from(3) * b.clone() + Expr::from(4) * Expr::from(2)) / (b.clone() + Expr::from(2)));
    }

    #[test]
    fn test_combine_like_terms() {
        // Test combining constant terms
        let terms1 = vec![
            CanonicalTerm::new(1),
            CanonicalTerm::new(2),
            CanonicalTerm::new(3),
        ];
        let result1 = CanonicalTerm::combine_like_terms(terms1);
        assert_eq!(result1.len(), 1);
        assert_eq!(result1[0].coef, Ratio::new(6, 1));
        assert!(result1[0].factors.is_empty());

        // Test combining terms with zero exponents
        let terms2 = vec![
            CanonicalTerm::new(1),
            CanonicalTerm {
                coef: Ratio::new(2, 1),
                factors: vec![
                    Factor { base: "a".to_string(), exponent: 0 },
                    Factor { base: "b".to_string(), exponent: 0 },
                ],
            },
            CanonicalTerm {
                coef: Ratio::new(3, 1),
                factors: vec![
                    Factor { base: "c".to_string(), exponent: 0 },
                ],
            },
        ];
        let result2 = CanonicalTerm::combine_like_terms(terms2);
        assert_eq!(result2.len(), 1);
        assert_eq!(result2[0].coef, Ratio::new(6, 1));
        assert!(result2[0].factors.is_empty());

        // Test combining constant terms with variable terms
        let terms3 = vec![
            CanonicalTerm::new(1),
            CanonicalTerm::with_var(2, "a".to_string()),
            CanonicalTerm::new(3),
            CanonicalTerm::with_var(4, "a".to_string()),
        ];
        let result3 = CanonicalTerm::combine_like_terms(terms3);
        assert_eq!(result3.len(), 2);
        assert_eq!(result3[0].coef, Ratio::new(4, 1));
        assert!(result3[0].factors.is_empty());
        assert_eq!(result3[1].coef, Ratio::new(6, 1));
        assert_eq!(result3[1].factors.len(), 1);
        assert_eq!(result3[1].factors[0].base, "a");
        assert_eq!(result3[1].factors[0].exponent, 1);

        // Test combining terms with zero coefficients
        let terms4 = vec![
            CanonicalTerm::new(1),
            CanonicalTerm::new(-1),
            CanonicalTerm::with_var(2, "a".to_string()),
            CanonicalTerm::with_var(-2, "a".to_string()),
        ];
        let result4 = CanonicalTerm::combine_like_terms(terms4);
        assert!(result4.is_empty());

        // Test combining terms with multiple variables
        let terms5 = vec![
            CanonicalTerm::new(1),
            CanonicalTerm::with_var(2, "a".to_string()),
            CanonicalTerm {
                coef: Ratio::new(1, 1),
                factors: vec![
                    Factor { base: "a".to_string(), exponent: 1 },
                    Factor { base: "b".to_string(), exponent: 1 },
                ],
            },
            CanonicalTerm {
                coef: Ratio::new(2, 1),
                factors: vec![
                    Factor { base: "a".to_string(), exponent: 1 },
                    Factor { base: "b".to_string(), exponent: 1 },
                ],
            },
        ];
        let result5 = CanonicalTerm::combine_like_terms(terms5);
        assert_eq!(result5.len(), 3);
        assert_eq!(result5[0].coef, Ratio::new(1, 1));
        assert!(result5[0].is_constant());
        assert_eq!(result5[1].coef, Ratio::new(2, 1));
        assert_eq!(result5[1].factors.len(), 1);
        assert_eq!(result5[1].factors[0].base, "a");
        assert_eq!(result5[2].coef, Ratio::new(3, 1));
        assert_eq!(result5[2].factors.len(), 2);
        assert_eq!(result5[2].factors[0].base, "a");
        assert_eq!(result5[2].factors[1].base, "b");

        // Test combining terms with mixed zero and non-zero exponents
        let terms6 = vec![
            CanonicalTerm {
                coef: Ratio::new(1, 1),
                factors: vec![
                    Factor { base: "a".to_string(), exponent: 0 },
                    Factor { base: "b".to_string(), exponent: 1 },
                ],
            },
            CanonicalTerm {
                coef: Ratio::new(2, 1),
                factors: vec![
                    Factor { base: "a".to_string(), exponent: 1 },
                    Factor { base: "b".to_string(), exponent: 0 },
                ],
            },
            CanonicalTerm {
                coef: Ratio::new(3, 1),
                factors: vec![
                    Factor { base: "a".to_string(), exponent: 0 },
                    Factor { base: "b".to_string(), exponent: 0 },
                ],
            },
        ];
        let result6 = CanonicalTerm::combine_like_terms(terms6);
        dbg!(&result6); 
        assert_eq!(result6.len(), 3);
        assert_eq!(result6[0].coef, Ratio::new(3, 1));
        assert!(result6[0].is_constant());
        assert_eq!(result6[1].coef, Ratio::new(2, 1));
        assert_eq!(result6[1].factors.len(), 1);
        assert_eq!(result6[1].factors[0].base, "a");
        assert_eq!(result6[2].coef, Ratio::new(1, 1));
        assert_eq!(result6[2].factors.len(), 1);
        assert_eq!(result6[2].factors[0].base, "b");
    }
}
