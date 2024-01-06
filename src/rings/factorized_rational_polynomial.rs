use std::{
    borrow::Cow,
    cmp::Ordering,
    fmt::{Display, Error, Formatter, Write},
    marker::PhantomData,
    ops::{Add, Div, Mul, Neg, Sub},
};

use crate::{
    poly::{
        factor::Factorize, gcd::PolynomialGCD, polynomial::MultivariatePolynomial, Exponent,
        Variable,
    },
    printer::{FactorizedRationalPolynomialPrinter, PrintOptions},
    state::State,
};

use super::{
    finite_field::{FiniteField, FiniteFieldCore, FiniteFieldWorkspace, ToFiniteField},
    integer::IntegerRing,
    rational::RationalField,
    EuclideanDomain, Field, Ring,
};

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct FactorizedRationalPolynomialField<R: Ring, E: Exponent> {
    ring: R,
    _phantom_exp: PhantomData<E>,
}

impl<R: Ring, E: Exponent> FactorizedRationalPolynomialField<R, E> {
    pub fn new(coeff_ring: R) -> FactorizedRationalPolynomialField<R, E> {
        FactorizedRationalPolynomialField {
            ring: coeff_ring,
            _phantom_exp: PhantomData,
        }
    }
}

pub trait FromNumeratorAndFactorizedDenominator<R: Ring, OR: Ring, E: Exponent> {
    fn from_num_den(
        num: MultivariatePolynomial<R, E>,
        dens: Vec<(MultivariatePolynomial<R, E>, usize)>,
        field: &OR,
        do_factor: bool,
    ) -> FactorizedRationalPolynomial<OR, E>;
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FactorizedRationalPolynomial<R: Ring, E: Exponent> {
    pub numerator: MultivariatePolynomial<R, E>,
    pub denominators: Vec<(MultivariatePolynomial<R, E>, usize)>, // TODO: sort factors?
}

impl<R: Ring, E: Exponent> PartialOrd for FactorizedRationalPolynomial<R, E> {
    /// An ordering of rational polynomials that has no intuitive meaning.
    fn partial_cmp(&self, _other: &Self) -> Option<Ordering> {
        todo!()
    }
}

impl<R: Ring, E: Exponent> FactorizedRationalPolynomial<R, E> {
    pub fn new(field: &R, var_map: Option<&[Variable]>) -> FactorizedRationalPolynomial<R, E> {
        let num = MultivariatePolynomial::new(
            var_map.map(|x| x.len()).unwrap_or(0),
            field,
            None,
            var_map,
        );
        let den = num.new_from_constant(field.one());

        FactorizedRationalPolynomial {
            numerator: num,
            denominators: vec![(den, 1)],
        }
    }

    pub fn get_var_map(&self) -> Option<&[Variable]> {
        self.numerator.var_map.as_ref().map(|x| x.as_slice())
    }

    pub fn unify_var_map(&mut self, other: &mut Self) {
        self.numerator.unify_var_map(&mut other.numerator);

        for d in &mut self.denominators {
            d.0.unify_var_map(&mut other.numerator);
        }
    }

    /// Constuct a pretty-printer for the rational polynomial.
    pub fn printer<'a, 'b>(
        &'a self,
        state: &'b State,
    ) -> FactorizedRationalPolynomialPrinter<'a, 'b, R, E> {
        FactorizedRationalPolynomialPrinter::new(self, state)
    }

    /// Convert the coefficient from the current field to a finite field.
    pub fn to_finite_field<UField: FiniteFieldWorkspace>(
        &self,
        field: &FiniteField<UField>,
    ) -> FactorizedRationalPolynomial<FiniteField<UField>, E>
    where
        R::Element: ToFiniteField<UField>,
        FiniteField<UField>: FiniteFieldCore<UField>,
        <FiniteField<UField> as Ring>::Element: Copy,
    {
        // check the gcd, since the rational polynomial may simplify
        FactorizedRationalPolynomial::from_num_den(
            self.numerator.to_finite_field(field),
            self.denominators
                .iter()
                .map(|(f, p)| (f.to_finite_field(field), *p))
                .collect(),
            field,
            true,
        )
    }
}

impl<E: Exponent> FromNumeratorAndFactorizedDenominator<RationalField, IntegerRing, E>
    for FactorizedRationalPolynomial<IntegerRing, E>
{
    fn from_num_den(
        num: MultivariatePolynomial<RationalField, E>,
        dens: Vec<(MultivariatePolynomial<RationalField, E>, usize)>,
        field: &IntegerRing,
        do_factor: bool,
    ) -> FactorizedRationalPolynomial<IntegerRing, E> {
        let mut content = num.content();
        for (d, _) in &dens {
            content = d.field.gcd(&content, &d.content());
        }

        let (num_int, dens_int) = if num.field.is_one(&content) {
            (
                num.map_coeff(|c| c.numerator(), IntegerRing::new()),
                dens.iter()
                    .map(|(d, p)| (d.map_coeff(|c| c.numerator(), IntegerRing::new()), *p))
                    .collect(),
            )
        } else {
            (
                num.map_coeff(
                    |c| num.field.div(&c, &content).numerator(),
                    IntegerRing::new(),
                ),
                dens.iter()
                    .map(|(d, p)| {
                        (
                            d.map_coeff(
                                |c| num.field.div(&c, &content).numerator(),
                                IntegerRing::new(),
                            ),
                            *p,
                        )
                    })
                    .collect(),
            )
        };

        <FactorizedRationalPolynomial<IntegerRing, E> as FromNumeratorAndFactorizedDenominator<
            IntegerRing,
            IntegerRing,
            E,
        >>::from_num_den(num_int, dens_int, field, do_factor)
    }
}

impl<E: Exponent> FromNumeratorAndFactorizedDenominator<IntegerRing, IntegerRing, E>
    for FactorizedRationalPolynomial<IntegerRing, E>
{
    fn from_num_den(
        mut num: MultivariatePolynomial<IntegerRing, E>,
        mut dens: Vec<(MultivariatePolynomial<IntegerRing, E>, usize)>, // TODO: we are assuming that all dens are irreducible
        _field: &IntegerRing,
        do_factor: bool,
    ) -> Self {
        for _ in 0..2 {
            for (d, _) in &mut dens {
                num.unify_var_map(d);
            }
        }

        // TODO: fuse constants

        if dens.len() == 1 && dens[0].0.is_one() {
            FactorizedRationalPolynomial {
                numerator: num,
                denominators: dens,
            }
        } else {
            if do_factor {
                for (d, _) in &mut dens {
                    let gcd = MultivariatePolynomial::gcd(&num, d);

                    if !gcd.is_one() {
                        num = num / &gcd;
                        *d = &*d / &gcd;
                    }
                }

                // factor all denominators, as they may be unfactored
                // TODO: add extra flag for this?
                let mut factored = vec![];
                for (d, p) in dens {
                    for (f, p2) in d.factor() {
                        factored.push((f, p * p2));
                    }
                }
                dens = factored;
            }

            // normalize denominator to have positive leading coefficient
            for (d, _) in &mut dens {
                if d.lcoeff().is_negative() {
                    num = -num;
                    *d = -d.clone(); // TODO: prevent
                }
            }

            let mut constant = num.new_from_constant(num.field.one());
            dens.retain(|f| {
                if f.0.is_constant() {
                    constant = &constant * &f.0.pow(f.1);
                    false
                } else {
                    true
                }
            });

            if dens.is_empty() || !constant.is_one() {
                dens.push((constant, 1));
            }

            FactorizedRationalPolynomial {
                numerator: num,
                denominators: dens,
            }
        }
    }
}

impl<UField: FiniteFieldWorkspace, E: Exponent>
    FromNumeratorAndFactorizedDenominator<FiniteField<UField>, FiniteField<UField>, E>
    for FactorizedRationalPolynomial<FiniteField<UField>, E>
where
    FiniteField<UField>: FiniteFieldCore<UField>,
    <FiniteField<UField> as Ring>::Element: Copy,
{
    fn from_num_den(
        mut num: MultivariatePolynomial<FiniteField<UField>, E>,
        mut dens: Vec<(MultivariatePolynomial<FiniteField<UField>, E>, usize)>,
        field: &FiniteField<UField>,
        do_factor: bool,
    ) -> Self {
        for _ in 0..2 {
            for (d, _) in &mut dens {
                num.unify_var_map(d);
            }
        }

        if dens.len() == 1 && dens[0].0.is_one() {
            FactorizedRationalPolynomial {
                numerator: num,
                denominators: dens,
            }
        } else {
            if do_factor {
                for (d, _) in &mut dens {
                    let gcd = MultivariatePolynomial::gcd(&num, d);

                    if !gcd.is_one() {
                        num = num / &gcd;
                        *d = &*d / &gcd;
                    }
                }

                // actor all denominators, as they may be unfactored
                // TODO: add extra flag for this?
                let mut factored = vec![];
                for (d, p) in dens {
                    for (f, p2) in d.factor() {
                        factored.push((f, p * p2));
                    }
                }
                dens = factored;
            }

            // normalize denominator to have leading coefficient of one
            for (d, _) in &mut dens {
                if !field.is_one(&d.lcoeff()) {
                    let c = d.lcoeff();
                    num = num.div_coeff(&c);
                    *d = d.clone().div_coeff(&c); // FIXME
                }
            }

            let mut constant = num.new_from_constant(num.field.one());
            dens.retain(|f| {
                if f.0.is_constant() {
                    constant = &constant * &f.0.pow(f.1);
                    false
                } else {
                    true
                }
            });

            if dens.is_empty() || !constant.is_one() {
                dens.push((constant, 1));
            }

            FactorizedRationalPolynomial {
                numerator: num,
                denominators: dens,
            }
        }
    }
}

impl<R: EuclideanDomain + PolynomialGCD<E>, E: Exponent> FactorizedRationalPolynomial<R, E>
where
    Self: FromNumeratorAndFactorizedDenominator<R, R, E>,
    MultivariatePolynomial<R, E>: Factorize,
{
    /// Invert a factored rational polynomial. This is an expensive operation, as it requires
    /// factoring the numerator.
    #[inline]
    pub fn inv(self) -> Self {
        if self.numerator.is_zero() {
            panic!("Cannot invert 0");
        }

        let mut num = self.numerator.new_from_constant(self.numerator.field.one());
        for (d, p) in self.denominators {
            num = num * &d.pow(p);
        }

        let dens = self.numerator.factor();

        let field = self.numerator.field.clone();
        Self::from_num_den(num, dens, &field, false)
    }
}

impl<R: EuclideanDomain + PolynomialGCD<E>, E: Exponent> FactorizedRationalPolynomial<R, E>
where
    Self: FromNumeratorAndFactorizedDenominator<R, R, E>,
{
    pub fn pow(&self, e: u64) -> Self {
        if e > u32::MAX as u64 {
            panic!("Power of exponentation is larger than 2^32: {}", e);
        }
        let e = e as u32;

        // TODO: do binary exponentation
        let mut poly = FactorizedRationalPolynomial {
            numerator: self.numerator.new_from_constant(self.numerator.field.one()),
            denominators: vec![(
                self.numerator.new_from_constant(self.numerator.field.one()),
                1,
            )],
        };

        for _ in 0..e {
            poly = &poly * self;
        }
        poly
    }

    pub fn gcd(&self, other: &Self) -> Self {
        let gcd_num = MultivariatePolynomial::gcd(&self.numerator, &other.numerator);

        let mut disjoint_factors = vec![];

        for (d, p) in &self.denominators {
            let mut found = false;
            for (d2, p2) in &other.denominators {
                if d == d2 {
                    disjoint_factors.push((d.clone(), *p.min(p2)));
                    found = true;
                    break;
                }
            }

            if !found {
                disjoint_factors.push((d.clone(), *p));
            }
        }

        for (d, p) in &other.denominators {
            let mut found = false;
            for (d2, _) in &self.denominators {
                if d == d2 {
                    found = true;
                    break;
                }
            }

            if !found {
                disjoint_factors.push((d.clone(), *p));
            }
        }

        FactorizedRationalPolynomial {
            numerator: gcd_num,
            denominators: disjoint_factors,
        }
    }
}

impl<R: Ring, E: Exponent> Display for FactorizedRationalPolynomial<R, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.denominators.len() == 1 && self.denominators[0].0.is_one() {
            self.numerator.fmt(f)
        } else {
            if f.sign_plus() {
                f.write_char('+')?;
            }

            f.write_fmt(format_args!("({})/(", self.numerator))?;

            for (d, p) in &self.denominators {
                if *p == 1 {
                    f.write_fmt(format_args!("{}", d))?;
                } else {
                    f.write_fmt(format_args!("({})^{}", d, p))?;
                }
            }

            f.write_char(')')
        }
    }
}

impl<R: Ring, E: Exponent> Display for FactorizedRationalPolynomialField<R, E> {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(()) // FIXME
    }
}

impl<R: EuclideanDomain + PolynomialGCD<E>, E: Exponent> Ring
    for FactorizedRationalPolynomialField<R, E>
where
    FactorizedRationalPolynomial<R, E>: FromNumeratorAndFactorizedDenominator<R, R, E>,
{
    type Element = FactorizedRationalPolynomial<R, E>;

    fn add(&self, a: &Self::Element, b: &Self::Element) -> Self::Element {
        a + b
    }

    fn sub(&self, a: &Self::Element, b: &Self::Element) -> Self::Element {
        // TODO: optimize
        self.add(a, &self.neg(b))
    }

    fn mul(&self, a: &Self::Element, b: &Self::Element) -> Self::Element {
        a * b
    }

    fn add_assign(&self, a: &mut Self::Element, b: &Self::Element) {
        // TODO: optimize
        *a = self.add(a, b);
    }

    fn sub_assign(&self, a: &mut Self::Element, b: &Self::Element) {
        *a = self.sub(a, b);
    }

    fn mul_assign(&self, a: &mut Self::Element, b: &Self::Element) {
        *a = self.mul(a, b);
    }

    fn add_mul_assign(&self, a: &mut Self::Element, b: &Self::Element, c: &Self::Element) {
        self.add_assign(a, &(b * c));
    }

    fn sub_mul_assign(&self, a: &mut Self::Element, b: &Self::Element, c: &Self::Element) {
        self.sub_assign(a, &(b * c));
    }

    fn neg(&self, a: &Self::Element) -> Self::Element {
        a.clone().neg()
    }

    fn zero(&self) -> Self::Element {
        FactorizedRationalPolynomial {
            numerator: MultivariatePolynomial::new(0, &self.ring, None, None),
            denominators: vec![(MultivariatePolynomial::one(&self.ring), 1)],
        }
    }

    fn one(&self) -> Self::Element {
        FactorizedRationalPolynomial {
            numerator: MultivariatePolynomial::one(&self.ring),
            denominators: vec![(MultivariatePolynomial::one(&self.ring), 1)],
        }
    }

    fn nth(&self, n: u64) -> Self::Element {
        FactorizedRationalPolynomial {
            numerator: MultivariatePolynomial::one(&self.ring).mul_coeff(self.ring.nth(n)),
            denominators: vec![(MultivariatePolynomial::one(&self.ring), 1)],
        }
    }

    fn pow(&self, b: &Self::Element, e: u64) -> Self::Element {
        if e > u32::MAX as u64 {
            panic!("Power of exponentation is larger than 2^32: {}", e);
        }
        let e = e as u32;

        // TODO: do binary exponentation
        let mut poly = FactorizedRationalPolynomial {
            numerator: b.numerator.new_from_constant(self.ring.one()),
            denominators: vec![(b.numerator.new_from_constant(self.ring.one()), 1)],
        };

        for _ in 0..e {
            poly = self.mul(&poly, b);
        }
        poly
    }

    fn is_zero(a: &Self::Element) -> bool {
        a.numerator.is_zero()
    }

    fn is_one(&self, a: &Self::Element) -> bool {
        a.numerator.is_one() && a.denominators.len() == 1 && a.denominators[0].0.is_one()
    }

    fn one_is_gcd_unit() -> bool {
        false
    }

    fn sample(&self, _rng: &mut impl rand::RngCore, _range: (i64, i64)) -> Self::Element {
        todo!("Sampling a polynomial is not possible yet")
    }

    fn fmt_display(
        &self,
        element: &Self::Element,
        state: Option<&State>,
        in_product: bool,
        f: &mut Formatter<'_>,
    ) -> Result<(), Error> {
        if f.sign_plus() {
            f.write_char('+')?;
        }

        if let Some(state) = state {
            f.write_fmt(format_args!(
                "{}",
                FactorizedRationalPolynomialPrinter {
                    poly: element,
                    state: state,
                    opts: PrintOptions::default(),
                    add_parentheses: in_product
                },
            ))
        } else if in_product {
            f.write_fmt(format_args!("({})", element))
        } else {
            f.write_fmt(format_args!("{}", element))
        }
    }
}

impl<R: EuclideanDomain + PolynomialGCD<E>, E: Exponent> EuclideanDomain
    for FactorizedRationalPolynomialField<R, E>
where
    FactorizedRationalPolynomial<R, E>: FromNumeratorAndFactorizedDenominator<R, R, E>,
    MultivariatePolynomial<R, E>: Factorize,
{
    fn rem(&self, a: &Self::Element, _: &Self::Element) -> Self::Element {
        FactorizedRationalPolynomial {
            numerator: MultivariatePolynomial::new_from(&a.numerator, None),
            denominators: vec![(a.numerator.new_from_constant(a.numerator.field.one()), 1)],
        }
    }

    fn quot_rem(&self, a: &Self::Element, b: &Self::Element) -> (Self::Element, Self::Element) {
        (self.div(a, b), self.zero())
    }

    fn gcd(&self, a: &Self::Element, b: &Self::Element) -> Self::Element {
        a.gcd(b)
    }
}

impl<R: EuclideanDomain + PolynomialGCD<E>, E: Exponent> Field
    for FactorizedRationalPolynomialField<R, E>
where
    FactorizedRationalPolynomial<R, E>: FromNumeratorAndFactorizedDenominator<R, R, E>,
    MultivariatePolynomial<R, E>: Factorize,
{
    fn div(&self, a: &Self::Element, b: &Self::Element) -> Self::Element {
        a / b
    }

    fn div_assign(&self, a: &mut Self::Element, b: &Self::Element) {
        *a = self.div(a, b);
    }

    fn inv(&self, a: &Self::Element) -> Self::Element {
        a.clone().inv()
    }
}

impl<'a, 'b, R: EuclideanDomain + PolynomialGCD<E> + PolynomialGCD<E>, E: Exponent>
    Add<&'a FactorizedRationalPolynomial<R, E>> for &'b FactorizedRationalPolynomial<R, E>
{
    type Output = FactorizedRationalPolynomial<R, E>;

    fn add(self, other: &'a FactorizedRationalPolynomial<R, E>) -> Self::Output {
        // (a/b + c/d) = (ad + bc)/bd

        let mut den = vec![];
        let mut num_1 = self.numerator.clone();

        let mut num_2 = other.numerator.clone();

        for (d, p) in &self.denominators {
            if let Some((_, p2)) = other.denominators.iter().find(|(d2, _)| d == d2) {
                if p > p2 {
                    num_2 = num_2 * &d.pow(*p - *p2);
                } else if p < p2 {
                    num_1 = num_1 * &d.pow(*p2 - *p);
                }
                den.push((d.clone(), *p.max(p2)));
                continue;
            }
            num_2 = num_2 * &d.pow(*p);
            den.push((d.clone(), *p));
        }
        for (d, p) in &other.denominators {
            if self.denominators.iter().any(|(d2, _)| d == d2) {
                continue;
            }
            num_1 = num_1 * &d.pow(*p);
            den.push((d.clone(), *p));
        }

        let mut num = num_1 + num_2;

        if num.is_zero() {
            den.clear();
            den.push((num.new_from_constant(num.field.one()), 1));
            return FactorizedRationalPolynomial {
                numerator: num,
                denominators: den,
            };
        }

        // TODO: are there some factors we can skip the division check for?

        for (d, p) in &mut den {
            while *p > 0 {
                let (q, r) = num.quot_rem(d, true);
                if !r.is_zero() {
                    break;
                }

                num = q;
                *p -= 1;
            }
        }

        den.retain(|(_, p)| *p > 0);

        let mut constant = num.new_from_constant(num.field.one());
        den.retain(|f| {
            if f.0.is_constant() {
                constant = &constant * &f.0.pow(f.1);
                false
            } else {
                true
            }
        });

        if !constant.is_one() {
            let g = num.field.gcd(&num.content(), &constant.coefficients[0]);
            if !num.field.is_one(&g) {
                num = num.div_coeff(&g);
                constant = constant.div_coeff(&g);
            }
        }

        if den.is_empty() || !constant.is_one() {
            den.push((constant, 1));
        }

        FactorizedRationalPolynomial {
            numerator: num,
            denominators: den,
        }
    }
}

impl<R: EuclideanDomain + PolynomialGCD<E>, E: Exponent> Sub
    for FactorizedRationalPolynomial<R, E>
{
    type Output = Self;

    fn sub(self, other: Self) -> Self::Output {
        self.add(&other.neg())
    }
}

impl<'a, 'b, R: EuclideanDomain + PolynomialGCD<E>, E: Exponent>
    Sub<&'a FactorizedRationalPolynomial<R, E>> for &'b FactorizedRationalPolynomial<R, E>
{
    type Output = FactorizedRationalPolynomial<R, E>;

    fn sub(self, other: &'a FactorizedRationalPolynomial<R, E>) -> Self::Output {
        (self.clone()).sub(other.clone())
    }
}

impl<R: EuclideanDomain + PolynomialGCD<E>, E: Exponent> Neg
    for FactorizedRationalPolynomial<R, E>
{
    type Output = Self;
    fn neg(self) -> Self::Output {
        FactorizedRationalPolynomial {
            numerator: self.numerator.neg(),
            denominators: self.denominators,
        }
    }
}

impl<'a, 'b, R: EuclideanDomain + PolynomialGCD<E>, E: Exponent>
    Mul<&'a FactorizedRationalPolynomial<R, E>> for &'b FactorizedRationalPolynomial<R, E>
{
    type Output = FactorizedRationalPolynomial<R, E>;

    fn mul(self, other: &'a FactorizedRationalPolynomial<R, E>) -> Self::Output {
        let mut reduced_numerator_1 = Cow::Borrowed(&self.numerator);
        let mut reduced_numerator_2 = Cow::Borrowed(&other.numerator);

        let mut den = vec![];

        for (d, p) in &other.denominators {
            if let Some((_, p2)) = self.denominators.iter().find(|(d2, _)| d == d2) {
                den.push((d.clone(), p + p2));
                continue;
            }

            if d.is_constant() {
                let g = reduced_numerator_1
                    .field
                    .gcd(&reduced_numerator_1.content(), &d.coefficients[0]);
                if reduced_numerator_1.field.is_one(&g) {
                    den.push((d.clone(), *p)); // p should be 1 here
                } else {
                    reduced_numerator_1 =
                        Cow::Owned(reduced_numerator_1.into_owned().div_coeff(&g));
                    den.push((d.clone().div_coeff(&g), *p));
                }
                continue;
            }

            let mut p = *p;
            while p > 0 {
                let (q, r) = reduced_numerator_1.quot_rem(d, true);
                if !r.is_zero() {
                    break;
                }

                reduced_numerator_1 = Cow::Owned(q);
                p -= 1;
            }

            if p > 0 {
                den.push((d.clone(), p));
            }
        }

        for (d, p) in &self.denominators {
            if other.denominators.iter().any(|(d2, _)| d == d2) {
                continue;
            }

            if d.is_constant() {
                let g = reduced_numerator_2
                    .field
                    .gcd(&reduced_numerator_2.content(), &d.coefficients[0]);
                if reduced_numerator_2.field.is_one(&g) {
                    den.push((d.clone(), *p)); // p should be 1 here
                } else {
                    reduced_numerator_2 =
                        Cow::Owned(reduced_numerator_2.into_owned().div_coeff(&g));
                    den.push((d.clone().div_coeff(&g), *p));
                }
                continue;
            }

            let mut p = *p;
            while p > 0 {
                let (q, r) = reduced_numerator_2.quot_rem(d, true);
                if !r.is_zero() {
                    break;
                }

                reduced_numerator_2 = Cow::Owned(q);
                p -= 1;
            }

            if p > 0 {
                den.push((d.clone(), p));
            }
        }

        let mut constant = reduced_numerator_1.new_from_constant(reduced_numerator_1.field.one());
        den.retain(|f| {
            if f.0.is_constant() {
                constant = &constant * &f.0.pow(f.1);
                false
            } else {
                true
            }
        });

        if den.is_empty() || !constant.is_one() {
            den.push((constant, 1));
        }

        FactorizedRationalPolynomial {
            numerator: reduced_numerator_1.as_ref() * reduced_numerator_2.as_ref(),
            denominators: den,
        }
    }
}

impl<'a, 'b, R: EuclideanDomain + PolynomialGCD<E>, E: Exponent>
    Div<&'a FactorizedRationalPolynomial<R, E>> for &'b FactorizedRationalPolynomial<R, E>
where
    FactorizedRationalPolynomial<R, E>: FromNumeratorAndFactorizedDenominator<R, R, E>,
    MultivariatePolynomial<R, E>: Factorize,
{
    type Output = FactorizedRationalPolynomial<R, E>;

    fn div(self, other: &'a FactorizedRationalPolynomial<R, E>) -> Self::Output {
        // TODO: optimize
        // the factored form can be kept intact and cancellations can be achieved before writing out the new numerator
        self * &other.clone().inv()
    }
}