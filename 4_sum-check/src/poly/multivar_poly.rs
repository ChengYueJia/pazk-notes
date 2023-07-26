use crate::poly::univar_poly::Polynomial;
use crate::utils::convert_to_binary;
use bls12_381::Scalar;
use ff::Field;
use log::{debug, log};
use std::collections::HashMap;
use std::hash::Hash;
use std::ops::{Add, AddAssign};

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
    pub fn evaluate(&self, domain: &Vec<usize>) -> Scalar {
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

    // Convert a muti-poly into a univariable-poly:
    //      f(x1, x2, x3, x4) , x1,x2,x3,x4 in hypercube
    //      With inputs(r1,r2,X,x4), the mpoly become a unipoly p(X)
    //
    // input: (r1, ..., r_{j-1}) in F,
    //        (x_j+1, ..., x_v} in hypercube{0,1}^v
    //
    // This is useful in sum-check protocol when obtaining g_i(X)
    pub fn partial_evaluate(&self, challenge_domain: &Vec<usize>) -> Polynomial {
        // the X = x_j, others has values.
        // Note here, x start with x_0, as the array index start with 0.
        let j = challenge_domain.len();
        assert!(j >= 0 || j < self.var_num);

        // <k,v>: k is the exp of X, v is the coeff, aka. <exp, coeff>
        let mut map: HashMap<usize, Scalar> = HashMap::new();

        // var_num = challenger_len + 1 + extra_len
        let extra_var_num = self.var_num - j - 1;
        let extra_n = 1 << extra_var_num;
        let extra_domain = (0..extra_n)
            .into_iter()
            .map(|n| convert_to_binary(&extra_var_num, n))
            .collect::<Vec<_>>();
        debug!(
            "extra domain {:?}, j {:?}, var_num:{:?}, extra_var_num: {:?}, extra_n: {:?}",
            extra_domain, j, self.var_num, extra_var_num, extra_n
        );

        // compute each term_i: coeff * product_x * X(x_j)
        for (index, coeff) in self.coeffs.iter().enumerate() {
            // if the coeff is 0, then skip it.
            if coeff.eq(&Scalar::zero()) {
                continue;
            }

            // x_0^exps[0] * x_1^exps[1] * x_2^exps[2]+ ...
            let exps = convert_to_binary(&self.var_num, index);

            // compute product_x on challenge_domain + hypercube_domain[i]
            for extra in extra_domain.clone() {
                // if index is 0, then term = coeff.
                if index == 0 {
                    map.entry(0)
                        .and_modify(|v| v.add_assign(&coeff.clone()))
                        .or_insert(coeff.clone());

                    continue;
                }

                // compute product of x , eg: product_x = (x_1^exp1) * (x_2^exp2), except x_j
                let mut key = 0;
                let mut product = 1;

                // evaluate on domain + hypercube_i
                let mut domain = challenge_domain.clone();
                domain.push(0);
                domain.extend(extra.clone());
                debug!(
                    "coeff:{:?}, domain:{:?}, j: {:?}, exps: {:?}",
                    coeff, domain, j, exps
                );
                for (index, (xi, exp)) in domain.iter().zip(exps.clone()).enumerate() {
                    if index == j {
                        key = exp.clone();
                    } else {
                        let pro = xi.pow(exp.clone() as u32);
                        debug!("x_{:?}^exp: {:?}^{:?}={:?}", index + 1, xi, exp, pro);
                        product *= pro;
                        // product *= xi.pow(exp.clone() as u32);
                    }
                    // once product, the computation of product is over. As zero multiple anything is zero.
                    if 0 == product {
                        break;
                    }
                }
                if 0 == product {
                    continue;
                } else {
                    let term_i = coeff.mul(&Scalar::from(product as u64));
                    debug!("k:{:?}, v:{:?}", key, term_i);
                    map.entry(key)
                        .and_modify(|v| v.add_assign(&term_i))
                        .or_insert(term_i);
                }
            }
        }

        // map -> poly
        // println!("map:{:?}", map);
        let max_key = map.keys().max().unwrap().clone();
        let coeffs = (0..=max_key)
            .map(|i: usize| {
                if map.contains_key(&i) {
                    map.get(&i).unwrap().clone()
                } else {
                    Scalar::zero()
                }
            })
            .collect::<Vec<_>>();
        Polynomial { coeffs }
    }
}

#[cfg(test)]
mod test {
    use crate::poly::multivar_poly::MPolynomial;
    use crate::poly::univar_poly::Polynomial;
    use crate::utils::convert_to_binary;
    use bls12_381::Scalar;
    use ff::PrimeField;

    #[test]
    fn test_partial_evaluate() {
        // let g(x1, x2, x3) = 5 + 2*x3 + 3*x2 +  x1 * x2 * x3
        // term0: exp: (0,0,0) = 5
        // term1: exp: (0,0,1) = 2*x3
        // term2: exp: (0,1,0) = 3*x2
        // term3-6: exp: (0,1,0) = 0.
        // term7: exp: (1,1,1) = x1 * x2 * x3

        let var_num = 3;

        let mpoly = MPolynomial {
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
        let challenge_domain = vec![10];

        let actual = mpoly.partial_evaluate(&challenge_domain);

        // expect t(x) = 12 + 16x
        let target = Polynomial {
            coeffs: vec![Scalar::from_u128(12), Scalar::from_u128(16)],
        };
        assert_eq!(actual, target);

        let actual_evaluation = actual.evaluate(Scalar::from_u128(10));

        let target_evaluation = Scalar::from_u128(172);
        assert_eq!(target_evaluation, actual_evaluation)
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
}
