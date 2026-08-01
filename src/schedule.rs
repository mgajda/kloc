//! Effort and schedule estimation models.
//!
//! All constants are from the primary literature:
//! - Basic COCOMO: Boehm, *Software Engineering Economics*, Prentice-Hall, 1981.
//! - COCOMO II: Boehm et al., *Software Cost Estimation with COCOMO II*,
//!   Prentice-Hall, 2000; USC-CSE COCOMO II Model Definition Manual v2.1.
//! - Putnam/SLIM: Putnam, "A General Empirical Solution to the Macro Software
//!   Sizing and Estimating Problem", *IEEE TSE* SE-4(4), 1978.
//! - Halstead: Halstead, *Elements of Software Science*, Elsevier, 1977.

/// Basic COCOMO (1981) project modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CocomoMode {
    Organic,
    SemiDetached,
    Embedded,
}

impl CocomoMode {
    const fn coefficients(self) -> (f64, f64, f64, f64) {
        match self {
            // (effort a, effort b, schedule c, schedule d)
            CocomoMode::Organic => (2.4, 1.05, 2.5, 0.38),
            CocomoMode::SemiDetached => (3.0, 1.12, 2.5, 0.35),
            CocomoMode::Embedded => (3.6, 1.20, 2.5, 0.32),
        }
    }

    pub fn effort_person_months(self, ksloc: f64) -> f64 {
        let (a, b, _, _) = self.coefficients();
        a * ksloc.powf(b)
    }

    pub fn schedule_months(self, person_months: f64) -> f64 {
        let (_, _, c, d) = self.coefficients();
        c * person_months.powf(d)
    }

    pub fn avg_people(self, person_months: f64) -> f64 {
        let months = self.schedule_months(person_months);
        if months > 0.0 { person_months / months } else { 0.0 }
    }
}

/// COCOMO II post-architecture model (2000), with all effort multipliers at
/// their nominal 1.00 and all scale factors at nominal.
///
/// PM = A · KSLOC^E · ∏EM,  E = B + 0.01·ΣSF.
/// With nominal inputs E = 0.91, ∏EM = 1 → PM = 2.94 · KSLOC^0.91.
pub fn cocomo_ii_person_months(ksloc: f64) -> f64 {
    // A = 2.94; E = B + 0.01·ΣSF at nominal scale factors = 0.91.
    const A: f64 = 2.94;
    const E: f64 = 0.91;
    A * ksloc.powf(E)
}

/// COCOMO II schedule: TDEV = C · PM_NS^F · (SCED/100), F = D + 0.2·(E−B).
/// Nominal scale factors and SCED=100% give F = 0.28.
pub fn cocomo_ii_schedule_months(person_months: f64) -> f64 {
    const C: f64 = 3.67;
    const F: f64 = 0.28;
    C * person_months.powf(F)
}

/// Putnam (1978) software equation, nominal schedule (from Putnam & Myers,
/// *Measures for Excellence*, 1991; Pressman's constants).
///
/// td = ( S³·B / (P³·K) )^(1/4), with S in ESLOC, K in person-years.
/// P = 28000 for business systems (the default), B = 0.16 small project.
pub fn putnam_schedule_years(esloc: f64, person_years: f64) -> f64 {
    const B: f64 = 0.16;
    const P: f64 = 28_000.0;
    let k = person_years.max(1e-9);
    (esloc.powi(3) * B / (P.powi(3) * k)).powf(0.25)
}

/// Halstead time to implement (seconds), from Halstead's *Elements of
/// Software Science* (1977).  Person-months conversion uses 152 h/PM.
pub fn halstead_person_months(effort: f64) -> f64 {
    const STROUD: f64 = 18.0; // mental discriminations per second
    const SECONDS_PER_PM: f64 = 152.0 * 3600.0;
    (effort / STROUD) / SECONDS_PER_PM
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ScheduleReport {
    pub ksloc: f64,
    pub cocomo: CocomoBreakdown,
    pub cocomo_ii: CocomoIiBreakdown,
    pub putnam: PutnamBreakdown,
    pub halstead: HalsteadBreakdown,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CocomoBreakdown {
    pub mode: CocomoMode,
    pub effort_person_months: f64,
    pub schedule_months: f64,
    pub avg_people: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CocomoIiBreakdown {
    pub effort_person_months: f64,
    pub schedule_months: f64,
    pub avg_people: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PutnamBreakdown {
    pub schedule_years: f64,
    pub schedule_months: f64,
    pub avg_people: f64,
}

/// Halstead's own schedule is single-developer (T = E/18 s), which is absurd
/// for large codebases. We reuse the COCOMO II schedule/effort relationship
/// (TDEV = C·PM^F, F = 0.28) to derive a parallelizable schedule and the
/// optimal team size that achieves it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HalsteadBreakdown {
    pub effort_person_months: f64,
    pub schedule_months: f64,
    pub avg_people: f64,
    /// The single-developer Halstead time (T = E/18 s), in seconds.
    pub single_developer_seconds: f64,
}

/// Build the schedule report from SLOC and Halstead effort.
pub fn estimate(sloc: u64, halstead_effort: f64) -> ScheduleReport {
    let ksloc = sloc as f64 / 1000.0;

    let mode = CocomoMode::Organic;
    let pm = mode.effort_person_months(ksloc);
    let cocomo = CocomoBreakdown {
        mode,
        effort_person_months: pm,
        schedule_months: mode.schedule_months(pm),
        avg_people: mode.avg_people(pm),
    };

    let pm2 = cocomo_ii_person_months(ksloc);
    let cocomo_ii = CocomoIiBreakdown {
        effort_person_months: pm2,
        schedule_months: cocomo_ii_schedule_months(pm2),
        avg_people: if cocomo_ii_schedule_months(pm2) > 0.0 {
            pm2 / cocomo_ii_schedule_months(pm2)
        } else { 0.0 },
    };

    let person_years = pm2 / 12.0;
    let py = putnam_schedule_years(sloc as f64, person_years);
    let putnam = PutnamBreakdown {
        schedule_years: py,
        schedule_months: py * 12.0,
        avg_people: if py > 0.0 { person_years / py } else { 0.0 },
    };

    // Halstead: single-developer time is absurd for large codebases, so we
    // derive a parallelizable schedule and optimal team size from the COCOMO II
    // schedule relationship.
    let halstead_pm = halstead_person_months(halstead_effort);
    let halstead = HalsteadBreakdown {
        effort_person_months: halstead_pm,
        schedule_months: cocomo_ii_schedule_months(halstead_pm),
        avg_people: if halstead_pm > 0.0 && cocomo_ii_schedule_months(halstead_pm) > 0.0 {
            halstead_pm / cocomo_ii_schedule_months(halstead_pm)
        } else { 0.0 },
        single_developer_seconds: halstead_effort / 18.0,
    };

    ScheduleReport {
        ksloc,
        cocomo,
        cocomo_ii,
        putnam,
        halstead,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cocomo_effort_monotonic() {
        // Effort and schedule must grow with size.
        let small = CocomoMode::Organic.effort_person_months(1.0);
        let large = CocomoMode::Organic.effort_person_months(100.0);
        assert!(large > small);
        let s_small = CocomoMode::Organic.schedule_months(small);
        let s_large = CocomoMode::Organic.schedule_months(large);
        assert!(s_large > s_small);
    }

    #[test]
    fn test_cocomo_avg_people_positive() {
        let pm = CocomoMode::Organic.effort_person_months(10.0);
        let avg = CocomoMode::Organic.avg_people(pm);
        assert!(avg > 0.0);
    }

    #[test]
    fn test_cocomo_ii_monotonic() {
        assert!(cocomo_ii_person_months(100.0) > cocomo_ii_person_months(1.0));
        assert!(cocomo_ii_schedule_months(100.0) > cocomo_ii_schedule_months(1.0));
    }

    #[test]
    fn test_putnam_finite_and_positive() {
        let y = putnam_schedule_years(1000.0, 2.0);
        assert!(y.is_finite() && y > 0.0);
        // More effort → shorter schedule.
        let y_more = putnam_schedule_years(1000.0, 200.0);
        assert!(y_more < y);
    }

    #[test]
    fn test_halstead_person_months() {
        // Zero effort → zero.
        assert_eq!(halstead_person_months(0.0), 0.0);
        // Larger effort → more person-months.
        assert!(halstead_person_months(1e6) > halstead_person_months(1e3));
    }

    #[test]
    fn test_estimate_sanity() {
        let r = estimate(10_000, 1e5);
        assert!((r.ksloc - 10.0).abs() < 1e-6);
        assert!(r.cocomo.effort_person_months > 0.0);
        assert!(r.cocomo_ii.effort_person_months > 0.0);
        assert!(r.putnam.schedule_months > 0.0);
        assert!(r.halstead.effort_person_months > 0.0);
        assert!(r.halstead.schedule_months > 0.0);
        assert!(r.halstead.avg_people > 0.0);
    }
}
