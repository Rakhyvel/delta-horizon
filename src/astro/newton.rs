use nalgebra_glm::{TMat, TVec};

/// A nonlinear problem with a fixed number of controls and residuals.
///
/// - `N_CONTROLS`: The number of optimization variables
/// - `N_RESIDUALS`: The number of residual constraints, to be targetted to 0
pub trait NLProblem<const N_CONTROLS: usize, const N_RESIDUALS: usize> {
    /// Computes the residual vector for some control variables.
    fn resid(&self, controls: &TVec<f64, N_CONTROLS>) -> TVec<f64, N_RESIDUALS>;

    /// Get the jacobian of the problem at an input `controls`
    ///
    /// Falls back to two-point finite differencing if not implemented analytically.
    fn jacobian(&self, controls: &TVec<f64, N_CONTROLS>) -> TMat<f64, N_RESIDUALS, N_CONTROLS> {
        let f0 = self.resid(controls);
        let mut j = TMat::<f64, N_RESIDUALS, N_CONTROLS>::zeros();

        for i in 0..N_CONTROLS {
            let mut perturbed: TVec<f64, N_CONTROLS> = *controls;
            let h = self.fd_step(i);
            perturbed[i] += h;

            let f1 = self.resid(&perturbed);
            j.set_column(i, &((f1 - f0) / h));
        }
        j
    }

    /// Finite-difference step size used for Jacobian approximation.
    fn fd_step(&self, _control_idx: usize) -> f64 {
        1e-6
    }
}

/// Solves a non-linear problem using a damped Newton method.
/// Returns the control vector `x` such that problem.resid(x) = 0
pub fn newton_target<const NC: usize, const NR: usize, P: NLProblem<NC, NR>>(
    problem: &P,
    initial_guess: TVec<f64, NC>,
    max_iter: usize,
    tol: f64,
    damping: f64,
) -> Result<TVec<f64, NC>, String> {
    let mut controls = initial_guess;

    for _ in 0..max_iter {
        let residual = problem.resid(&controls);
        let norm = residual.norm();

        if norm < tol {
            return Ok(controls);
        }

        let j = problem.jacobian(&controls);

        // Least-squares solve: (JᵀJ) dx = Jᵀ r
        let jtj = j.transpose() * j;
        let jtr = j.transpose() * residual;

        let dx = jtj.try_inverse().ok_or("singular jacobian")? * jtr;

        controls -= damping * dx;
    }

    Err(format!(
        "newton_target did not converge after {max_iter} iterations"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra_glm::{TMat, TVec};

    /// Simple linear system: Ax = b
    /// residual = Ax - b, solution is x = A^{-1}b
    /// Analytic jacobian is just A
    struct LinearProblem {
        a: TMat<f64, 2, 2>,
        b: TVec<f64, 2>,
    }

    impl NLProblem<2, 2> for LinearProblem {
        fn resid(&self, controls: &TVec<f64, 2>) -> TVec<f64, 2> {
            self.a * controls - self.b
        }

        fn jacobian(&self, _controls: &TVec<f64, 2>) -> TMat<f64, 2, 2> {
            self.a // analytic — exact
        }
    }

    #[test]
    fn test_linear_analytic_jacobian() {
        let problem = LinearProblem {
            a: TMat::<f64, 2, 2>::new(2.0, 1.0, 1.0, 3.0),
            b: TVec::<f64, 2>::new(5.0, 10.0),
        };

        // Known solution: x = [1, 3]
        let guess = TVec::<f64, 2>::new(0.0, 0.0);
        let sol = newton_target(&problem, guess, 50, 1e-10, 1.0).unwrap();

        assert!((sol[0] - 1.0).abs() < 1e-8, "x[0] wrong: {}", sol[0]);
        assert!((sol[1] - 3.0).abs() < 1e-8, "x[1] wrong: {}", sol[1]);
    }

    #[test]
    fn test_linear_fd_jacobian() {
        /// Same problem but jacobian falls back to finite differences
        struct LinearProblemFD {
            a: TMat<f64, 2, 2>,
            b: TVec<f64, 2>,
        }

        impl NLProblem<2, 2> for LinearProblemFD {
            fn resid(&self, controls: &TVec<f64, 2>) -> TVec<f64, 2> {
                self.a * controls - self.b
            }
            // no jacobian override, uses FD
        }

        let problem = LinearProblemFD {
            a: TMat::<f64, 2, 2>::new(2.0, 1.0, 1.0, 3.0),
            b: TVec::<f64, 2>::new(5.0, 10.0),
        };

        let guess = TVec::<f64, 2>::new(0.0, 0.0);
        let sol = newton_target(&problem, guess, 50, 1e-8, 1.0).unwrap();

        assert!((sol[0] - 1.0).abs() < 1e-6, "x[0] wrong: {}", sol[0]);
        assert!((sol[1] - 3.0).abs() < 1e-6, "x[1] wrong: {}", sol[1]);
    }

    #[test]
    fn test_fd_jacobian_matches_analytic() {
        let problem = LinearProblem {
            a: TMat::<f64, 2, 2>::new(2.0, 1.0, 1.0, 3.0),
            b: TVec::<f64, 2>::new(5.0, 10.0),
        };

        let controls = TVec::<f64, 2>::new(1.0, 2.0);

        // Compute both jacobians
        let j_analytic = problem.jacobian(&controls);
        let j_fd = {
            // Call the default FD impl by going through a wrapper that doesn't override jacobian
            struct FDWrapper<'a>(&'a LinearProblem);
            impl NLProblem<2, 2> for FDWrapper<'_> {
                fn resid(&self, c: &TVec<f64, 2>) -> TVec<f64, 2> {
                    self.0.resid(c)
                }
                // no jacobian override
            }
            FDWrapper(&problem).jacobian(&controls)
        };

        let diff = (j_analytic - j_fd).abs();
        assert!(
            diff.max() < 1e-5,
            "FD jacobian differs from analytic:\n{diff}"
        );
    }

    /// Nonlinear: find x,y such that x^2 + y^2 = 1 and x = y
    /// residual = [x^2 + y^2 - 1, x - y]
    /// solution = [1/sqrt(2), 1/sqrt(2)] or [-1/sqrt(2), -1/sqrt(2)]
    struct CircleLineProblem;

    impl NLProblem<2, 2> for CircleLineProblem {
        fn resid(&self, c: &TVec<f64, 2>) -> TVec<f64, 2> {
            TVec::<f64, 2>::new(c[0] * c[0] + c[1] * c[1] - 1.0, c[0] - c[1])
        }
        // FD jacobian
    }

    #[test]
    fn test_nonlinear_fd() {
        let problem = CircleLineProblem;
        let guess = TVec::<f64, 2>::new(1.0, 0.5); // near [1/sqrt(2), 1/sqrt(2)]
        let sol = newton_target(&problem, guess, 50, 1e-10, 1.0).unwrap();

        let expected = 1.0_f64 / 2.0_f64.sqrt();
        assert!((sol[0] - expected).abs() < 1e-8, "x wrong: {}", sol[0]);
        assert!((sol[1] - expected).abs() < 1e-8, "y wrong: {}", sol[1]);

        // Verify residual is actually zero
        let resid = problem.resid(&sol);
        assert!(resid.norm() < 1e-8, "residual not zero: {resid}");
    }

    #[test]
    fn test_nonconvergence_returns_err() {
        // Unsolvable: x^2 + y^2 = -1
        struct ImpossibleProblem;
        impl NLProblem<2, 2> for ImpossibleProblem {
            fn resid(&self, c: &TVec<f64, 2>) -> TVec<f64, 2> {
                TVec::<f64, 2>::new(c[0] * c[0] + c[1] * c[1] + 1.0, 0.0)
            }
        }

        let result = newton_target(
            &ImpossibleProblem,
            TVec::<f64, 2>::new(0.0, 0.0),
            10,
            1e-10,
            1.0,
        );
        assert!(result.is_err(), "should have failed to converge");
    }
}
