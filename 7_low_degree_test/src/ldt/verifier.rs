use crate::ldt::LDTProof;
use crate::merkle_tree::proof::MerkleProof;
use crate::merkle_tree::MerkleTree;
use crate::transcript::default::Keccak256Transcript;
use crate::transcript::Transcript;
use bls12_381::Scalar;

pub struct Verifier {
    pub target_deg: usize, // target degree
    z: Scalar,
    merkle_c: Scalar,
}

impl Verifier {
    pub fn verify(&self, d: usize, proof: &LDTProof) {
        let mut transcript = Keccak256Transcript::default();

        let commits = proof.commits;
        let evals = proof.evals;
        let (const_L, const_R) = proof.last_const;

        for i in 0..d {
            // 1. obtain fi_L(z^2) ,z fi_R (z^2) with: fi(z) = fi_L(z^2) + z fi_R (z^2)
            let (f_i_z, f_i_neg_z) = evals.get(i).unwrap();
            let f_i_L = (f_i_z + f_i_neg_z) / 2;
            let f_i_R = (f_i_z - f_i_neg_z) / 2;
            // assert_eq!(
            //     f_i_z,
            //     f_i_L + z * f_i_R,
            //     "Fail to check fi(z) = fi_L(z^2) + z fi_R (z^2)"
            // );

            if (d - 1) == i {
                // 2. last round check: last_const == (*p_L.coeffs().get(0).unwrap(), *p_R.coeffs().get(0).unwrap());
                assert!(f_i_L.degree() == 0 && f_i_R.degree() == 0);
                assert_eq!(
                    proof.last_const,
                    (*p_L.coeffs().get(0).unwrap(), *p_R.coeffs().get(0).unwrap()),
                    "Verifier: Last round check failed."
                );
            } else {
                // 2. check fi+1(z^2) = fi_L(z^2) + αi*fi_R(z^2)
                let alpha = transcript.challenge();
                let f_i_plus_1 = f_i_L + alpha * f_i_R;
                let (target_f_i_plus_1, _) = evals.get(i + 1).unwrap();
                assert_eq!(
                    f_i_plus_1, target_f_i_plus_1,
                    "Verifier: round-{i} check failed."
                );

                // 3. verify the cm
                MerkleTree::verify(&self.merkle_c, commits[&i]);
            }
        }
    }
}
