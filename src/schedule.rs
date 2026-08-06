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
        if months > 0.0 {
            person_months / months
        } else {
            0.0
        }
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
/// for large codebases. Derive a parallelizable schedule and team size from
/// the COCOMO II relationship (TDEV = C·PM^F, F = 0.28) instead.
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
    let sm2 = cocomo_ii_schedule_months(pm2);
    let cocomo_ii = CocomoIiBreakdown {
        effort_person_months: pm2,
        schedule_months: sm2,
        avg_people: if sm2 > 0.0 { pm2 / sm2 } else { 0.0 },
    };

    let person_years = pm2 / 12.0;
    let py = putnam_schedule_years(sloc as f64, person_years);
    let putnam = PutnamBreakdown {
        schedule_years: py,
        schedule_months: py * 12.0,
        avg_people: if py > 0.0 { person_years / py } else { 0.0 },
    };

    let halstead_pm = halstead_person_months(halstead_effort);
    let hsm = cocomo_ii_schedule_months(halstead_pm);
    let halstead = HalsteadBreakdown {
        effort_person_months: halstead_pm,
        schedule_months: hsm,
        avg_people: if halstead_pm > 0.0 && hsm > 0.0 {
            halstead_pm / hsm
        } else {
            0.0
        },
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

    #[test]
    fn test_all_cocomo_modes_coefficients() {
        // Exercise every mode's coefficients tuple.
        for mode in [
            CocomoMode::Organic,
            CocomoMode::SemiDetached,
            CocomoMode::Embedded,
        ] {
            let pm = mode.effort_person_months(50.0);
            let sched = mode.schedule_months(pm);
            assert!(pm > 0.0 && sched > 0.0);
            assert!(mode.avg_people(pm) > 0.0);
            assert!(mode.avg_people(0.0) == 0.0);
        }
    }
    #[test]
    fn cocomo_exact_values() {
        assert_eq!(
            CocomoMode::Organic.effort_person_months(10.0),
            26.928442903247127
        );
        assert_eq!(
            CocomoMode::Organic.schedule_months(26.928442903247127),
            8.738165793274268
        );
        assert_eq!(
            CocomoMode::SemiDetached.effort_person_months(10.0),
            39.547702156692225
        );
        assert_eq!(
            CocomoMode::SemiDetached.schedule_months(39.547702156692225),
            9.055917300087549
        );
        assert_eq!(
            CocomoMode::Embedded.effort_person_months(10.0),
            57.05615492860008
        );
        assert_eq!(
            CocomoMode::Embedded.schedule_months(57.05615492860008),
            9.119201433458992
        );
        // avg_people = pm / schedule; zero pm yields zero, not NaN.
        assert_eq!(CocomoMode::Organic.avg_people(0.0), 0.0);
        assert_eq!(
            CocomoMode::Organic.avg_people(26.928442903247127),
            3.081704277569767
        );
    }

    #[test]
    fn cocomo_ii_exact_values() {
        assert_eq!(cocomo_ii_person_months(1.0), 2.94);
        assert_eq!(cocomo_ii_person_months(10.0), 23.89721717522452);
        assert_eq!(
            cocomo_ii_schedule_months(23.89721717522452),
            8.92489913801881
        );
        assert_eq!(cocomo_ii_schedule_months(1.0), 3.67);
    }

    #[test]
    fn putnam_and_halstead_exact() {
        assert_eq!(putnam_schedule_years(1000.0, 2.0), 0.043692206064732314);
        assert_eq!(halstead_person_months(1e6), 0.10152696556205328);
        assert_eq!(halstead_person_months(0.0), 0.0);
    }

    #[test]
    fn estimate_pins_all_breakdowns() {
        let r = estimate(10_000, 1e5);
        assert_eq!(r.ksloc, 10.0);
        assert_eq!(r.cocomo.effort_person_months, 26.928442903247127);
        assert_eq!(r.cocomo.schedule_months, 8.738165793274268);
        assert_eq!(r.cocomo.avg_people, 3.081704277569767);
        assert_eq!(r.cocomo_ii.effort_person_months, 23.89721717522452);
        assert_eq!(r.cocomo_ii.schedule_months, 8.92489913801881);
        assert_eq!(r.cocomo_ii.avg_people, 2.6775896069711025);
        assert_eq!(r.putnam.schedule_years, 0.2459630960476584);
        assert_eq!(r.putnam.schedule_months, 2.9515571525719007);
        assert_eq!(r.putnam.avg_people, 8.096477872502378);
        assert_eq!(r.halstead.effort_person_months, 0.010152696556205328);
        assert_eq!(r.halstead.schedule_months, 1.0151000706079572);
        assert_eq!(r.halstead.avg_people, 0.010001670623591565);
    }

    #[test]
    fn estimate_zero_is_finite() {
        // Zero SLOC and zero effort must not produce NaN anywhere.
        let r = estimate(0, 0.0);
        assert_eq!(r.cocomo_ii.avg_people, 0.0);
        assert_eq!(r.putnam.avg_people, 0.0);
        assert_eq!(r.halstead.avg_people, 0.0);
        assert!(r.putnam.schedule_months.is_finite());
        assert!(r.cocomo_ii.schedule_months.is_finite());
    }
}
