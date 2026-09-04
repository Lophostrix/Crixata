use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Grade {
    A,
    B,
    C,
    D,
}

impl std::fmt::Display for Grade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Grade::A => write!(f, "A"),
            Grade::B => write!(f, "B"),
            Grade::C => write!(f, "C"),
            Grade::D => write!(f, "D"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRights {
    pub can_delete_data: bool,
    pub can_export_data: bool,
    pub can_opt_out_of_tracking: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicySummary {
    pub data_collected: Vec<String>,
    pub data_used_for: Vec<String>,
    pub shared_with_third_parties: bool,
    pub third_party_details: String,
    pub retention_period: String,
    pub user_rights: UserRights,
    pub tracking_and_ads: String,
    pub arbitration_or_class_action_waiver: bool,
    pub policy_clarity_notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradeRequest {
    pub url: String,
    pub title: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradeResponse {
    pub grade: Grade,
    pub summary: PolicySummary,
    pub cached: bool,
    pub source: String,
}

/// Computes a deterministic privacy grade A-D from the extracted structured summary
/// based on the rubric defined in docs/ARCHITECTURE.md.
pub fn compute_grade(summary: &PolicySummary) -> Grade {
    let mut score: i32 = 100;

    // 1. Third-party data sharing penalty
    if summary.shared_with_third_parties {
        score -= 25;
    }

    // 2. Forced arbitration / class action waiver penalty
    if summary.arbitration_or_class_action_waiver {
        score -= 30;
    }

    // 3. User rights penalties
    if !summary.user_rights.can_delete_data {
        score -= 20;
    }
    if !summary.user_rights.can_opt_out_of_tracking {
        score -= 15;
    }
    if !summary.user_rights.can_export_data {
        score -= 10;
    }

    // 4. Data collection breadth penalty
    if summary.data_collected.len() > 6 {
        score -= 15;
    } else if summary.data_collected.len() > 3 {
        score -= 5;
    }

    match score {
        85.. => Grade::A,
        65..=84 => Grade::B,
        45..=64 => Grade::C,
        _ => Grade::D,
    }
}
