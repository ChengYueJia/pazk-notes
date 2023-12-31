use bls12_381::Scalar;
use ff::Field;
use rand_core::OsRng;
use std::ops::AddAssign;

/// This define `matrix` (rows * cols) （m × n）
#[derive(Debug, Clone)]
pub struct Matrix {
    rows: usize,
    cols: usize,
    // columns
    values: Vec<Vec<Scalar>>,
}

impl Matrix {
    pub fn random(rows: usize, cols: usize) -> Self {
        let values = (0..rows)
            .map(|_| (0..cols).map(|_| Scalar::random(OsRng)).collect::<Vec<_>>())
            .collect::<Vec<_>>();

        Self { cols, rows, values }
    }

    fn get_columns(&self, column_index: usize) -> Vec<Scalar> {
        assert!(self.cols > column_index);

        self.values
            .iter()
            .map(|v| v.get(column_index).unwrap().clone())
            .collect::<Vec<_>>()
    }

    fn vec_mul(a: &Vec<Scalar>, b: &Vec<Scalar>) -> Scalar {
        assert_eq!(a.len(), b.len());

        let mut res = Scalar::zero();
        for (ai, bi) in a.into_iter().zip(b) {
            let producti = ai.mul(bi);
            res.add_assign(producti);
        }
        res
    }

    /// https://en.wikipedia.org/wiki/Dot_product
    /// Suppose A(m * n), x(n) => A * x = y(n)
    pub fn matrix_mul_vec(&self, vector: &Vec<Scalar>) -> Vec<Scalar> {
        assert_eq!(self.cols, vector.len());
        let n = self.cols;

        let mut result: Vec<Scalar> = Vec::with_capacity(n);
        for i in 0..self.rows {
            let row_i = self.values.get(i).unwrap().clone();

            let elem = Self::vec_mul(&row_i, vector);

            result.push(elem);
        }

        result
    }

    /// https://en.wikipedia.org/wiki/Dot_product
    /// Suppose A(m * n), B(n, p) => A * B = C(m * p)
    pub fn mul(m_a: &Matrix, m_b: &Matrix) -> Self {
        assert!(m_a.cols > 0 || m_b.rows > 0, "matrix a is empty");
        assert!(m_b.cols > 0 || m_b.rows > 0, "matrix a is empty");
        // ma.cols == mb.rows
        assert_eq!(m_a.cols, m_b.rows);
        let m = m_a.rows;
        let n = m_a.cols;
        // let n = m_b.rows;
        let p = m_b.cols;

        let mut matrix: Vec<Vec<Scalar>> = Vec::with_capacity(m);
        for i in 0..m {
            let mut new_row = Vec::with_capacity(p);

            let row_i = m_a.values.get(i).unwrap().clone();
            for j in 0..p {
                // todo: this can be optimized by converting m_b columns as rows
                let col_j = m_b.get_columns(j);
                let elem_ij = Self::vec_mul(&row_i, &col_j);
                new_row.push(elem_ij);
            }

            matrix.push(new_row);
        }

        Self {
            rows: m,
            cols: p,
            values: matrix,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::matrix::Matrix;
    use bls12_381::Scalar;
    use ff::PrimeField;

    #[test]
    fn test_random_matrix() {
        let matrix = Matrix::random(3, 4);
        println!("{:#?}", matrix);
    }

    #[test]
    fn test_matrix_mul() {
        let m: usize = 2;
        let mut values: Vec<Vec<Scalar>> = Vec::with_capacity(m);
        let mut row_1: Vec<Scalar> = Vec::with_capacity(m);
        row_1.push(Scalar::one());
        row_1.push(Scalar::zero());
        let mut row_2: Vec<Scalar> = Vec::with_capacity(m);
        row_2.push(Scalar::zero());
        row_2.push(Scalar::one());
        values.push(row_1);
        values.push(row_2);

        let a = Matrix {
            rows: m,
            cols: m,
            values,
        };
        let b = a.clone();

        let res = Matrix::mul(&a, &b);
        assert_eq!(a.values, res.values);
        println!("{:#?}", res);
    }

    #[test]
    fn test_matrix_mul_vec() {
        let m: usize = 2;
        let mut values: Vec<Vec<Scalar>> = Vec::with_capacity(m);
        let mut row_1: Vec<Scalar> = Vec::with_capacity(m);
        row_1.push(Scalar::one());
        row_1.push(Scalar::zero());
        let mut row_2: Vec<Scalar> = Vec::with_capacity(m);
        row_2.push(Scalar::zero());
        row_2.push(Scalar::one());
        values.push(row_1.clone());
        values.push(row_2);

        let a = Matrix {
            rows: m,
            cols: m,
            values,
        };
        let b = a.clone();

        let res = a.matrix_mul_vec(&row_1);
        assert_eq!(row_1, res);
        println!("{:#?}", res);
    }

    #[test]
    fn test() {
        let n = 2;
        let A = Matrix::random(n, n);
        let B = Matrix::random(n, n);
        let x = vec![Scalar::from_u128(3), Scalar::from_u128(5)];

        // A*B*x
        let res1 = Matrix::mul(&A, &B).matrix_mul_vec(&x);
        // A*(B*x)
        let res2 = A.matrix_mul_vec(&B.matrix_mul_vec(&x));
        assert_eq!(res1, res2);
    }
}
