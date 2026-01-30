use hypors::chi_square;
// use heapless::vec
pub struct MatchingEstimator {
    pub expected_power_spectrum: Vec<f32>,
    max_shift: usize,
}

impl MatchingEstimator {
    pub fn new(
        expected_power_spectrum: Vec<f32>,
        max_shift: usize,
    ) -> Self {
        Self {
            expected_power_spectrum,
            max_shift,
        }
    }

    // Basic - I want to try a smarter method for the edges later to compare full frame instead of cutting out either side
    pub fn sliding_window_match(&mut self, current_power_spectrum: &mut Vec<f32>) -> (f32, usize) {
        // let expected_mean: f32 = expected_average.iter().sum();
        // let expected_den: f32 = expected_average.iter().map(|x| (x - expected_mean).powi(2)).sum::<f32>().sqrt();
        let current_average = current_power_spectrum;
        let expected_average = &mut self.expected_power_spectrum;
        let max_shift = &self.max_shift;
        let match_len = expected_average.len() - max_shift;

        let shift: usize = max_shift / 2;
        let current_start = shift;
        let current_end = current_average.len() - shift;
        let sliding_window = &current_average[current_start..current_end];

        let window_mean: f32 = sliding_window.iter().sum::<f32>() / sliding_window.len() as f32;
        let window_den: f32 = sliding_window
            .iter()
            .map(|bin| (bin - window_mean).powi(2))
            .sum::<f32>()
            .sqrt();

        let mut best_score = -100.0;
        let mut best_index = 0;
        let mut res: f32 = 0.0;
        for i in 0..=*max_shift {
            let compare_slice = &expected_average[i..i + match_len];
            let compare_mean: f32 = compare_slice.iter().sum::<f32>() / compare_slice.len() as f32;
            let compare_den: f32 = compare_slice
                .iter()
                .map(|bin| (bin - compare_mean).powi(2))
                .sum::<f32>()
                .sqrt();

            for (compare_bin, window_bin) in compare_slice.iter().zip(sliding_window.iter()) {
                res += (compare_bin - compare_mean) * (window_bin - window_mean);
            }
            res /= compare_den * window_den;

            if res > best_score {
                best_score = res;
                best_index = i;
            }
            res = 0.0;
        }
        return (best_score, best_index);
    }

    // Rework to handle divide by zero
    pub fn chi_square(&mut self, best_score: f32, best_index: usize, current_power_spectrum: &Vec<f32>) -> f32 {
        let shift: usize = &self.max_shift / 2;
        let current_start = shift;
        let current_end = current_power_spectrum.len() - shift;
        let match_len = current_power_spectrum.len() - &self.max_shift;

        let mut window = &current_power_spectrum[current_start..current_end];
        let window_norm = 1.0 / window.iter().sum::<f32>();
        let window: Vec<f32> = window.iter().map(|bin| bin * window_norm).collect();
        let mut compare_slice = &self.expected_power_spectrum[best_index..best_index + match_len];
        let compare_norm = 1.0 / compare_slice.iter().sum::<f32>();
        let compare_slice: Vec<f32> = compare_slice.iter().map(|bin| bin * compare_norm).collect();

        let alpha = 0.05;
        let chi_sqr =
            chi_square::goodness_of_fit(window.clone(), compare_slice.clone(), alpha).unwrap();

        println!("Chi-square: {}", chi_sqr.test_statistic);

        return chi_sqr.test_statistic as f32;
    }

    pub fn sliding_window_advanced(&mut self, current_power_spectrum: &Vec<f32>) -> (f32, isize) {
        let shift: isize = (&self.max_shift / 2) as isize;

        let full_len = current_power_spectrum.len();

        let mut best_score = -100.0;
        let mut best_index = 0;
        let mut res: f32 = 0.0;

        for i in -shift..=shift {
            let (window_start, window_end) = if i < 0 {
                (i.abs() as usize, full_len)
            } else {
                (0, full_len - i as usize)
            };
            let (compare_start, compare_end) = if i < 0 {
                (0, (full_len as isize + i) as usize)
            } else {
                (i as usize, full_len)
            };

            let window_slice = &current_power_spectrum[window_start..window_end];
            let compare_slice = &self.expected_power_spectrum[compare_start..compare_end];

            let window_mean: f32 = window_slice.iter().sum::<f32>() / window_slice.len() as f32;
            let window_den: f32 = window_slice
                .iter()
                .map(|bin| (bin - window_mean).powi(2))
                .sum::<f32>()
                .sqrt();

            let compare_mean: f32 = compare_slice.iter().sum::<f32>() / compare_slice.len() as f32;
            let compare_den: f32 = compare_slice
                .iter()
                .map(|bin| (bin - compare_mean).powi(2))
                .sum::<f32>()
                .sqrt();

            for (compare_bin, window_bin) in compare_slice.iter().zip(window_slice.iter()) {
                res += (compare_bin - compare_mean) * (window_bin - window_mean);
            }
            res /= compare_den * window_den;

            if res > best_score {
                best_score = res;
                best_index = i;
            }
            res = 0.0;
        }
        return (best_score, best_index);
    }

    pub fn chi_square_advanced(&mut self, best_score: f32, best_index: isize, current_power_spectrum: &Vec<f32>) -> f32 {
        let full_len = current_power_spectrum.len();

        let (window_start, window_end) = if best_index < 0 {
            (best_index.abs() as usize, full_len)
        } else {
            (0, full_len - best_index as usize)
        };
        let (compare_start, compare_end) = if best_index < 0 {
            (0, (full_len as isize + best_index) as usize)
        } else {
            (best_index as usize, full_len)
        };

        let window_slice = &current_power_spectrum[window_start..window_end];
        let compare_slice = &self.expected_power_spectrum[compare_start..compare_end];

        // Normalize
        let window_norm = 1.0 / window_slice.iter().sum::<f32>();
        let window_slice: Vec<f32> = window_slice.iter().map(|bin| bin * window_norm).collect();

        let compare_norm = 1.0 / compare_slice.iter().sum::<f32>();
        let compare_slice: Vec<f32> = compare_slice.iter().map(|bin| bin * compare_norm).collect();

        let alpha = 0.05;
        let chi_sqr =
            chi_square::goodness_of_fit(window_slice.clone(), compare_slice.clone(), alpha)
                .unwrap();

        println!("Advanced Chi-square: {}", chi_sqr.test_statistic);

        return chi_sqr.test_statistic as f32;
    }

    // Q = e ^ (-x^2)/2 ~ Abandoned for now because it did not draw a strong dis
    // fn quality_estimate(&self, chi_square: f32) -> f32 {
    //     let k = 0.5;
    //     (-k * chi_square).exp()
    // }

    pub fn sigmoid(&self, chi_square: f32) -> f32 {
        // If chi square is above 2.2 the exponentiated value is a large positive number
        // leading to division to almost 0 for estimate

        // 2.2 is the boundary for evaluation of 0.5 (uncertain)

        // if chi square is below 2.2, the exponentiated value is a small positive number
        // causing the result to be 1 / 1 + almost 1
        let cutoff = 0.5;
        let slope = 2.0;
        1.0 / (1.0 + (slope * (chi_square - cutoff)).exp())
    }

    pub fn match_estimate(&mut self, current_power_spectrum: &mut Vec<f32>) {
        let (best_score, best_index) = self.sliding_window_match(current_power_spectrum);
        let chi_score = self.chi_square(best_score, best_index, current_power_spectrum);
    }

    pub fn match_estimate_advanced(&mut self, current_power_spectrum: &mut Vec<f32>) -> f32 {
        let (best_score, best_index) = self.sliding_window_advanced(current_power_spectrum);
        let chi_square = self.chi_square_advanced(best_score, best_index, current_power_spectrum);
        // self.quality_estimate(chi_square)
        self.sigmoid(chi_square)
    }
}
