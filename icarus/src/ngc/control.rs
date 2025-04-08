use nalgebra::{Matrix6x1, Matrix6};

struct SpacecraftLQR{
    a_matrix: Matrix6<f64>,
    b_matrix: Matrix6<f64>,
    u_matrix: Matrix6x1<f64>,
    state: Matrix6x1,
}

impl SpacecraftLQR{
    pub fn new(a_matrix: Matrix6, b_matrix: Matrix6, u_matrix: Matrix6x1, state_initial: Matrix6x1) -> Self {
        Control {
            a_matrix,
            b_matrix,
            u_matrix,
            state: state_initial,
        }
    }
    pub fn set_a_matrix(&mut self, a_matrix: Matrix6) {
        self.a_matrix = a_matrix;
    }
    pub fn set_b_matrix(&mut self, b_matrix: Matrix6) {
        self.b_matrix = b_matrix;
    }
    pub fn set_u_matrix(&mut self, u_matrix: Matrix6x1) {
        self.u_matrix = u_matrix;
    }
    pub fn calculate_state(&self, dt: f64) -> Matrix6x1<f64> {
        let state_change = self.a_matrix * self.state + self.b_matrix * self.u_matrix;
        let desired_input = solve_riccati(&self.a_matrix, &self.b_matrix, &self.q_matrix, &self.r_matrix, dt);
        if let Some((p, k)) = desired_input {
            self.u_matrix = -k * self.state;
        } else {
            self.u_matrix = self.u_matrix; // Keep the previous control input if the Riccati equation did not converge
        }
        // Update the state (Trapezoidal integration)
        self.state += 0.5 * (self.state + state_change) * dt;
    }

    pub fn solve_riccati(a: &Matrix6<f64>, b: &Matrix6<f64>, q: &Matrix6<f64>, r: &Matrix6<f64>, dt: f64) -> Option<(DMatrix<f64>, DMatrix<f64>)> {
        let tolerance = 1e-5;
        let max_iter = 100_000;
    
        let mut p = q.clone();
        let mut diff = f64::MAX;
        let mut iter = 0;
    
        let r_inv = r.clone().try_inverse()?;
        let bt = b.transpose();
        let at = a.transpose();
    
        while diff > tolerance && iter < max_iter {
            iter += 1;
            let p_next = &p + (&at * &p + &p * a + q - &p * b * &r_inv * &bt * &p) * dt;
            diff = (&p_next - &p).amax();
            p = p_next;
        }
    
        if diff > tolerance {
            return None; // convergence not achieved
        }
    
        let k = &r_inv * &bt * &p;
        Some((p, k))
    }
    
}

