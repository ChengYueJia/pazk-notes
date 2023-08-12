use crate::gkr_sumcheck::F_r_Poly;
use crate::poly::{MPolynomial, Polynomial};
use crate::utils::convert_to_binary;
use bls12_381::Scalar;
use std::ops::{Add, Mul};
use std::path::Iter;

pub struct Prover {
    v_l: usize, // the constants_part var_num.  v_l + v_r = ki + 2*k_i_plus_1
    v_r: usize, // the variable_part var_num. equals to `v` in standard sumcheck.
    add: MPolynomial,
    mult: MPolynomial,
    w_i_plus_1: MPolynomial,
    r_i: Vec<usize>, // the constant var part.
}

impl Prover {
    pub fn new(
        (v_l, v_r): (usize, usize),
        (add, mult, w_i_plus_1): F_r_Poly,
        r_i: Vec<usize>,
    ) -> Self {
        Self {
            v_l,
            v_r,
            add,
            mult,
            w_i_plus_1,
            r_i,
        }
    }

    // obtain m1 by $\sum_{b,c \in (0,1)^{k_{i+1}}}f_{r_i} = m_i $ , m1 means C1.
    pub fn proof(&self) -> Scalar {
        let k_i_plus_1 = self.w_i_plus_1.var_num;

        let mut res = Scalar::zero();
        for i in 0..k_i_plus_1 {
            let a = convert_to_binary(&k_i_plus_1, i);
            let w_a = self.w_i_plus_1.evaluate(&a);

            // ops_domain = (ri, a, b)
            let mut ops_domain = self.r_i.clone();
            ops_domain.append(&mut a.clone());

            for j in 0..k_i_plus_1 {
                let b = convert_to_binary(&k_i_plus_1, j);

                let w_b = self.w_i_plus_1.evaluate(&b);

                ops_domain.clone().append(&mut b.clone());
                let add_i = self.add.evaluate(&ops_domain);
                let multi = self.mult.evaluate(&ops_domain);

                res += add_i * (w_a + w_b) + multi * (w_a * w_b);
            }
        }
        res
    }

    // Return g1(X) = sum g(X, x_2, ..., x_v)
    // obtain  $g1(X) =  = m_i $ , m1 means C1.
    // Return g1(x) = add(r_i, (X, a2, ...,a_k_1), (b1, ..., b_k_1) * (W(X, a2, ...,a_k_1) + W(b1,...,b_k_1))
    //              + mult(r_i, (X, a2, ...,a_k_1), (b1, ..., b_k_1) * (W(X, a2, ...,a_k_1) * W(b1,...,b_k_1))
    //              = poly_add * (poly_w_a + w_b) + poly_mult * (poly_w_a * w_b)
    //              = poly_add * poly_w_a + poly_add * w_b + poly_mult * (poly_w_a * w_b)
    pub fn round_1(&self) -> Polynomial {
        let poly_add = self.add.partial_evaluate(&self.r_i);
        let poly_mult = self.mult.partial_evaluate(&self.r_i);

        let poly_w_a = self.w_i_plus_1.partial_evaluate(&vec![]);
        let w_b = self.w_i_plus_1.sum_all_evals();

        // poly_add * poly_w_a + poly_add * w_b + poly_mult * (poly_w_a * w_b)
        poly_add
            .mul(&poly_w_a)
            .add(&poly_add.mul(&w_b).add(&poly_mult.mul(&poly_w_a).mul(&w_b)))
    }

    // 1 < j < v_r, total v_r-2 rounds
    // Return g_j = (r1, ..., r_j-1, X, x_j+1, ..., x_v)
    pub fn recursive_round_j(&self, challenges: &Vec<usize>) -> Polynomial {
        assert!(self.v_r > challenges.len() || challenges.len() >= 1);

        // partial_evaluate with (r_i, challenge, X, x_i)
        let mut ops_challenge_domain = self.r_i.clone();
        ops_challenge_domain.append(&mut challenges.clone());
        let poly_add = self.add.partial_evaluate(&ops_challenge_domain);
        let poly_mult = self.mult.partial_evaluate(&ops_challenge_domain);

        let poly_w_a = self.w_i_plus_1.partial_evaluate(challenges);
        let w_b = self.w_i_plus_1.sum_all_evals();

        // poly_add * poly_w_a + poly_add * w_b + poly_mult * (poly_w_a * w_b)
        poly_add
            .mul(&poly_w_a)
            .add(&poly_add.mul(&w_b).add(&poly_mult.mul(&poly_w_a).mul(&w_b)))
    }

    // Return g_v = (r1, r2, ..., r_v-1, X_v)
    pub fn round_v(&self, challenges: &Vec<usize>) -> Polynomial {
        assert_eq!(self.v_r - 1, challenges.len());

        // partial_evaluate with (r_i, challenge, X, x_i)
        let mut ops_challenge_domain = self.r_i.clone();
        ops_challenge_domain.append(&mut challenges.clone());
        let poly_add = self.add.partial_evaluate(&ops_challenge_domain);
        let poly_mult = self.mult.partial_evaluate(&ops_challenge_domain);

        let poly_w_a = self.w_i_plus_1.partial_evaluate(challenges);
        let w_b = self.w_i_plus_1.sum_all_evals();

        // poly_add * poly_w_a + poly_add * w_b + poly_mult * (poly_w_a * w_b)
        poly_add
            .mul(&poly_w_a)
            .add(&poly_add.mul(&w_b).add(&poly_mult.mul(&poly_w_a).mul(&w_b)))
    }

    // pub fn evaluate(&self, challenges: &Vec<usize>) -> Scalar {
    //     self.g.evaluate(challenges)
    // }
}
