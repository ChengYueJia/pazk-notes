// Generally speaking, there are two type of implements:
// # Impl-1
// ## Struct
//
// pub struct MSparsePolynomial<F: Field, T: Term> {
//     /// The number of variables the polynomial supports
//     #[derivative(PartialEq = "ignore")]
//     pub num_vars: usize,
//     /// List of each term along with its coefficient
//     /// term: (coeff, T)
//     /// T: [(var_index, exp)]
//     pub terms: Vec<(F, T)>,
// }
//
// ## Examples
// `2*x_0^3 + x_0*x_2 + x_1*x_2 + 5`:
//
// let poly = MSparsePolynomial::from_coefficients_vec(
//     3,
//     vec![
//         (Fq::from(2), SparseTerm::new(vec![(0, 3)])),
//         (Fq::from(1), SparseTerm::new(vec![(0, 1), (2, 1)])),
//         (Fq::from(1), SparseTerm::new(vec![(1, 1), (2, 1)])),
//         (Fq::from(5), SparseTerm::new(vec![])),
//     ],
// );
//
// # Impl-2
// Multivariate polynomials are represented as hash maps with exponent vectors
// as keys and coefficients as values. E.g.:
// ## struct
// pub struct MPolynomial<T: FiniteField> {
//
//     pub variable_count: usize,
//     // Notice that the exponent values may not exceed 0xFF = 255 = u8::MAX.
//     pub coefficients: HashMap<Vec<u8>, T>,
// }
//
// ## Examples
// f(x,y,z) = 17 + 2xy + 42z - 19x^6*y^3*z^12 is represented as:
// var_num = 3,
//     {
//         [0,0,0] => 17,
//         [1,1,0] => 2,
//         [0,0,1] => 42,
//         [6,3,12] => -19,
//     }
use crate::utils::{convert_from_binary, convert_to_binary, expand_factor_for_mpoly};
use bls12_381::Scalar;
use ff::Field;
use std::collections::HashMap;
use std::env::var;
use std::ops::AddAssign;

// A multivariate polynomial g is multilinear if the degree of the polynomial in each variable is at most one.
// For example, the polynomial g(x1,x2) = x_1*x_2 +4x_1 +3x_2 is multilinear, but the polynomial
// h(x1,x2) = x2 + 4x1 + 3x2 is not.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MPolynomial {
    pub var_num: usize,
    // The index (with binary form) is the exponent values.
    pub coeffs: Vec<Scalar>,
}

impl MPolynomial {
    // w: {0,1}^v
    // F(x_1,...,x_v) = ∑f(w)·X_w(x_1,...,x_v),
    // X_w(x1,...,xv) := ∏(xiwi +(1−xi)(1−wi)).
    fn lagrange(var_num: usize, evals: &Vec<Scalar>) -> Self {
        let n: usize = 1 << var_num;
        assert_eq!(evals.len(), n, "Domain is less than var_num");

        let mut F = vec![Scalar::zero(); n];

        // compute f_i = f_w * X_w
        for (i, f_w) in evals.iter().enumerate() {
            let w_i = convert_to_binary(&var_num, i);
            // X_w(x1,...,xv) := ∏(xiwi +(1−xi)(1−wi)).
            let X_w = Self::mpoly_langrange_basis(var_num, w_i);
            // f_i = f(w)·X_w
            let f_i = X_w.iter().map(|X_w_i| X_w_i * f_w).collect::<Vec<_>>();

            // F = ∑f_j
            for i in 0..n {
                F[i].add_assign(f_i[i]);
            }
        }
        Self { var_num, coeffs: F }
    }

    // X_w(x1,...,xv) := ∏(xiwi +(1−xi)(1−wi)).
    // eg: F(x1,x2) =>
    //           X_w(0, 0) = (1−x1) * (1−x2)
    //           X_w(0, 1) = (1−x1) * x2
    //           X_w(1, 0) = x1 * (1−x2)
    //           X_w(1, 1) = x1 * x2
    // Though X_w is little complex, world will be changed when w in {0,1}^v hypercube.
    // It's easy to cauth:
    //      wi = 1, (xiwi +(1−xi)(1−wi))= xi ;
    //      wi = 0, (xiwi +(1−xi)(1−wi))= (1 - xi) ;
    // So it's easy to obtain the factorization form of X_w.
    // eg: if var_num = 4, w=(0, 0, 1, 1), so that X_w(0,0,1,1)=(1-x_1)(1-x_2) * x_3 * x_4
    fn mpoly_langrange_basis(var_num: usize, w: Vec<usize>) -> Vec<Scalar> {
        assert_eq!(var_num, w.len());
        let poly_len = 1 << var_num;

        // eg: if var_num = 4, w=(0, 0, 1, 1), so that X_w(0,0,1,1)=(1-x_1)(1-x_2) * x_3 * x_4
        // factors as below:
        //      inputs        => xi => term exp     = term coeff
        //      (i=0, w1 = 0) => x1 => (1, 0, 0, 0) = -1
        //      (i=1, w2 = 0) => x2 => (0, 1, 0, 0) = -1
        //      (i=2, w3 = 1) => x3 => (0, 0, 1, 0) = 1
        //      (i=3, w4 = 1) => x4 => (0, 0, 0, 1) = 1
        let gen_X_wi = |i: usize, w_i: usize| {
            let mut factor = vec![Scalar::zero(); poly_len];

            // For (i=0, w1 = 0) => x1, whose coeff exp is (1, 0, 0, 0).
            // We need to encode it into index for coeff vector.
            let index: usize = 1 << (var_num - 1 - i);
            match w_i {
                0 => {
                    factor[0] = Scalar::one();
                    factor[index] = Scalar::one().neg();
                }
                1 => {
                    factor[index] = Scalar::one();
                }
                _ => panic!("Only support (0,1)^v hypercube"),
            }
            // println!("index:{:?}, w_i:{:?}", index, w_i);
            // println!("factor_i: {:?}", factor);
            factor
        };

        // init with w[0].
        let mut product = gen_X_wi(0, w[0]);

        for (i, w_i) in w.iter().enumerate() {
            if i == 0 {
                continue;
            }
            let factor = gen_X_wi(i, w_i.clone());
            product = expand_factor_for_mpoly(var_num, product, factor);
        }

        product
    }

    fn evaluate(&self, domain: &Vec<usize>) -> Scalar {
        assert_eq!(domain.len(), self.var_num, "Domain is less than var_num");

        let mut sum_of_term = Scalar::zero();

        // compute each term_i: coeff * product_x
        for (index, coeff) in self.coeffs.iter().enumerate() {
            // if the coeff is 0, then skip it.
            if coeff.eq(&Scalar::zero()) {
                continue;
            }

            // if index is 0, then term = coeff.
            if index == 0 {
                sum_of_term += coeff;
            } else {
                // x_0^exps[0] * x_1^exps[1] * x_2^exps[2]+ ...
                let exps = convert_to_binary(&self.var_num, index);

                // compute product of x , eg: product_x = (x_1^exp1) * (x_2^exp2)
                let mut product = 1;
                for (x_i, exp_i) in domain.into_iter().zip(exps) {
                    let x = x_i.clone();

                    // Note, as the definition, the exp is in [0, 1]
                    // if exp != 0 && x != 0 {
                    product *= x.pow(exp_i as u32);

                    // once product, the computation of product is over. As zero multiple anything is zero.
                    if 0 == product {
                        break;
                    }
                }

                match product {
                    0 => continue,
                    1 => sum_of_term += coeff,
                    _ => {
                        let term_i = coeff.mul(&Scalar::from(product as u64));
                        sum_of_term.add_assign(term_i);
                    }
                }
            }
        }
        sum_of_term
    }
}

// TODO impl Fmt for mpoly

#[cfg(test)]
mod test {
    use crate::utils::*;
    use crate::MPolynomial::MPolynomial;
    use bls12_381::Scalar;
    use ff::PrimeField;

    #[test]
    fn test_lagrange() {
        // let row g(x1, x2, x3) = 5 + 2*x3 + 3*x2 +  x1 * x2 * x3
        // term0: exp: (0,0,0) = 5
        // term1: exp: (0,0,1) = 2*x3
        // term2: exp: (0,1,0) = 3*x2
        // term3-6: exp: (0,1,0) = 0.
        // term7: exp: (1,1,1) = x1 * x2 * x3

        let var_num = 3;

        let evals = vec![
            Scalar::from_u128(5),
            Scalar::from_u128(2),
            Scalar::from_u128(3),
            Scalar::zero(),
            Scalar::zero(),
            Scalar::zero(),
            Scalar::zero(),
            Scalar::one(),
        ];

        let poly = MPolynomial::lagrange(var_num, &evals);

        // all domains
        let max_num: usize = 1 << var_num;
        let domains = (0..max_num)
            .into_iter()
            .map(|n| convert_to_binary(&var_num, n))
            .collect::<Vec<_>>();

        let actual = domains
            .iter()
            .map(|domain| poly.evaluate(domain))
            .collect::<Vec<_>>();
        assert_eq!(evals, actual);
        println!("poly: {:?}", poly);
    }

    #[test]
    fn test_mpoly_langrange_basis() {
        // eg: if var_num = 4, w=(0, 0, 1, 1),
        // so that X_w(0,0,1,1)=(1-x_1)(1-x_2) * x_3 * x_4
        //             = x_3*x_4 - x_1*x_3*x_4 - x_2*x_3*x_4 + x_1*x_2*x_3*x_4
        // term3: exp: (0,0,1,1) = x_3*x_4
        // term7: exp: (0,1,1,1) = -x_2*x_3*x_4
        // term11: exp: (1,0,1,1) = -x_1*x_3*x_4
        // term15: exp: (1,1,1,1) = x_1*x_2*x_3*x_4
        // other term = 0

        let var_num = 4;
        let n = 1 << var_num;
        let w = vec![0, 0, 1, 1];
        let mut target = vec![Scalar::zero(); n];
        target[3] = Scalar::one();
        target[7] = Scalar::one().neg();
        target[11] = Scalar::one().neg();
        target[15] = Scalar::one();

        let actual = MPolynomial::mpoly_langrange_basis(var_num, w);
        assert_eq!(actual, target);
    }

    #[test]
    fn test_2_mpoly_langrange_basis() {
        // eg: if var_num = 2, w=(0,1),
        // so that X_w(0,0,1,1)=(1−x1) * x2
        //             = x2 - x1*x2
        // term0: exp: (0,0) = 0
        // term1: exp: (0,1) = x2
        // term2: exp: (1,0) = 0
        // term3: exp: (1,1) = - x1*x2

        let var_num = 2;
        let n = 1 << var_num;
        let w = vec![0, 1];
        let target = vec![
            Scalar::zero(),
            Scalar::one(),
            Scalar::zero(),
            Scalar::one().neg(),
        ];

        let actual = MPolynomial::mpoly_langrange_basis(var_num, w);
        assert_eq!(actual, target);
    }

    #[test]
    fn test_evaluate() {
        // let g(x1, x2, x3) = 5 + 2*x3 + 3*x2 +  x1 * x2 * x3
        // term0: exp: (0,0,0) = 5
        // term1: exp: (0,0,1) = 2*x3
        // term2: exp: (0,1,0) = 3*x2
        // term3-6: exp: (0,1,0) = 0.
        // term7: exp: (1,1,1) = x1 * x2 * x3

        let var_num = 3;

        let poly = MPolynomial {
            var_num,
            coeffs: vec![
                Scalar::from_u128(5),
                Scalar::from_u128(2),
                Scalar::from_u128(3),
                Scalar::zero(),
                Scalar::zero(),
                Scalar::zero(),
                Scalar::zero(),
                Scalar::one(),
            ],
        };

        // domain: (0,1,1)
        let domain = convert_to_binary(&var_num, 3);
        let target = Scalar::from_u128(10);

        let actual = poly.evaluate(&domain);
        assert_eq!(target, actual);
    }

    #[test]
    fn test_domain() {
        // g(x1,...,xv) = x1*x2 + 4*x1 + 3*x2 + ... + xv
        // var_num: v
        // domain: [[0; v], ..., [0, 1,..., 0], ..., [1; v]]
        let var_num: Vec<usize> = vec![2, 3, 4];

        for num in var_num.iter() {
            let max_num: usize = 1 << num;
            let domain = (0..max_num)
                .into_iter()
                .map(|n| convert_to_binary(&num, n))
                .collect::<Vec<_>>();
            assert_eq!(domain.len(), max_num);
            println!("num: {:?}", num);
            println!("domain: size:{:?},  {:?}", domain.len(), domain);
        }
    }
}
