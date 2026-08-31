//! Post-certification runtime metric carriers and policy projections.
//! A gate/tier label is a derived consumer observation, not the fine runtime carrier.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeTimingVector {
    pub model_cold_load_ns: u64,
    pub parser_wall_occupancy_ns: u64,
    pub sensiblaw_active_ns: u64,
    pub total_pipeline_wall_ns: u64,
    pub external_controller_wall_ns: u64,
    pub post_parser_tail_ns: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExactRatio {
    pub numerator: u64,
    pub denominator: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassingTier {
    Architectural2x,
    Production1_5x,
    Production1_2x,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PerformanceProjection {
    pub gate_passed: bool,
    pub tier: Option<PassingTier>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionError {
    ZeroParserTime,
}

impl RuntimeTimingVector {
    pub fn parser_relative_ratio(&self) -> Result<ExactRatio, ProjectionError> {
        if self.parser_wall_occupancy_ns == 0 {
            return Err(ProjectionError::ZeroParserTime);
        }
        Ok(ExactRatio {
            numerator: self.total_pipeline_wall_ns,
            denominator: self.parser_wall_occupancy_ns,
        })
    }

    pub fn performance_projection(&self) -> Result<PerformanceProjection, ProjectionError> {
        let parser = self.parser_wall_occupancy_ns as u128;
        if parser == 0 {
            return Err(ProjectionError::ZeroParserTime);
        }
        let total = self.total_pipeline_wall_ns as u128;

        let tier = if total.saturating_mul(10) <= parser.saturating_mul(12) {
            Some(PassingTier::Production1_2x)
        } else if total.saturating_mul(2) <= parser.saturating_mul(3) {
            Some(PassingTier::Production1_5x)
        } else if total <= parser.saturating_mul(2) {
            Some(PassingTier::Architectural2x)
        } else {
            None
        };

        Ok(PerformanceProjection {
            gate_passed: tier.is_some(),
            tier,
        })
    }
}

/// Exact timing vector from the certified GWB v0.1 receipt at Rust head
/// 60777f637732f28fed46458a30853d35b88a8a09.
pub const GWB_V01_CERTIFIED_TIMING: RuntimeTimingVector = RuntimeTimingVector {
    model_cold_load_ns: 701_777_110,
    parser_wall_occupancy_ns: 127_919_406_353,
    sensiblaw_active_ns: 1_135_911_693,
    total_pipeline_wall_ns: 136_058_451_205,
    external_controller_wall_ns: 136_067_579_483,
    post_parser_tail_ns: 7_611_429,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn certified_gwb_projection_is_one_point_two() {
        assert_eq!(
            GWB_V01_CERTIFIED_TIMING.performance_projection(),
            Ok(PerformanceProjection {
                gate_passed: true,
                tier: Some(PassingTier::Production1_2x),
            })
        );
        assert_eq!(
            GWB_V01_CERTIFIED_TIMING.parser_relative_ratio(),
            Ok(ExactRatio {
                numerator: 136_058_451_205,
                denominator: 127_919_406_353,
            })
        );
    }

    #[test]
    fn same_projection_does_not_mean_same_timing_vector() {
        let a = RuntimeTimingVector {
            model_cold_load_ns: 10,
            parser_wall_occupancy_ns: 100,
            sensiblaw_active_ns: 1,
            total_pipeline_wall_ns: 110,
            external_controller_wall_ns: 111,
            post_parser_tail_ns: 1,
        };
        let b = RuntimeTimingVector {
            model_cold_load_ns: 40,
            parser_wall_occupancy_ns: 100,
            sensiblaw_active_ns: 8,
            total_pipeline_wall_ns: 110,
            external_controller_wall_ns: 119,
            post_parser_tail_ns: 9,
        };
        assert_ne!(a, b);
        assert_eq!(a.performance_projection(), b.performance_projection());
        assert_eq!(
            a.performance_projection().unwrap().tier,
            Some(PassingTier::Production1_2x)
        );
    }

    #[test]
    fn zero_parser_time_is_not_silently_classified() {
        let timing = RuntimeTimingVector {
            model_cold_load_ns: 0,
            parser_wall_occupancy_ns: 0,
            sensiblaw_active_ns: 0,
            total_pipeline_wall_ns: 1,
            external_controller_wall_ns: 1,
            post_parser_tail_ns: 1,
        };
        assert_eq!(timing.performance_projection(), Err(ProjectionError::ZeroParserTime));
    }
}
