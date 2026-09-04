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

impl From<char> for Grade {
    fn from(c: char) -> Self {
        match c.to_ascii_uppercase() {
            'A' => Grade::A,
            'B' => Grade::B,
            'C' => Grade::C,
            _ => Grade::D,
        }
    }
}

impl From<Grade> for char {
    fn from(g: Grade) -> Self {
        match g {
            Grade::A => 'A',
            Grade::B => 'B',
            Grade::C => 'C',
            Grade::D => 'D',
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserRights {
    pub can_delete_data: bool,
    pub can_export_data: bool,
    pub can_opt_out_of_tracking: bool,
}

impl Default for UserRights {
    fn default() -> Self {
        Self {
            can_delete_data: true,
            can_export_data: true,
            can_opt_out_of_tracking: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Summary {
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

pub type PolicySummary = Summary;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradeRequest {
    pub url: String,
    #[serde(default)]
    pub title: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradeResponse {
    pub grade: char,
    pub summary: Summary,
    pub cached: bool,
    pub source: String,
}

/// Computes a deterministic privacy grade A-D from the extracted structured summary
/// based on the rubric defined in docs/ARCHITECTURE.md.
///
/// Rubric:
/// - A (>= 85): Strong privacy (minimal collection, no 3rd-party sharing, clear rights, no arbitration waiver)
/// - B (65..=84): Generally fair (some tracking or sharing, but rights exist and clarity is reasonable)
/// - C (45..=64): Concerning (broad collection, significant sharing, weak rights, or forced arbitration)
/// - D (< 45): Poor (excessive collection, no deletion/export, arbitration/class-action waiver)
pub fn compute_grade(summary: &Summary) -> char {
    let mut score: i32 = 100;

    // 1. Forced arbitration / class action waiver penalty
    if summary.arbitration_or_class_action_waiver {
        score -= 30;
    }

    // 2. Third-party data sharing penalty
    if summary.shared_with_third_parties {
        score -= 25;
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
        85.. => 'A',
        65..=84 => 'B',
        45..=64 => 'C',
        _ => 'D',
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perfect_policy_gets_a() {
        let summary = Summary {
            data_collected: vec!["email".into()],
            data_used_for: vec!["authentication".into()],
            shared_with_third_parties: false,
            third_party_details: "None".into(),
            retention_period: "Until account deletion".into(),
            user_rights: UserRights {
                can_delete_data: true,
                can_export_data: true,
                can_opt_out_of_tracking: true,
            },
            tracking_and_ads: "None".into(),
            arbitration_or_class_action_waiver: false,
            policy_clarity_notes: "Clear and concise".into(),
        };

        assert_eq!(compute_grade(&summary), 'A');
    }

    #[test]
    fn test_moderate_sharing_gets_b() {
        let summary = Summary {
            data_collected: vec!["email".into(), "ip_address".into()],
            data_used_for: vec!["service provision".into(), "analytics".into()],
            shared_with_third_parties: true, // -25 -> 75
            third_party_details: "Analytics partners".into(),
            retention_period: "12 months".into(),
            user_rights: UserRights {
                can_delete_data: true,
                can_export_data: true,
                can_opt_out_of_tracking: true,
            },
            tracking_and_ads: "Analytics cookies only".into(),
            arbitration_or_class_action_waiver: false,
            policy_clarity_notes: "Fair terms".into(),
        };

        assert_eq!(compute_grade(&summary), 'B');
    }

    #[test]
    fn test_arbitration_and_sharing_gets_c_or_d() {
        let summary = Summary {
            data_collected: vec![
                "email".into(),
                "ip".into(),
                "device_id".into(),
                "location".into(),
            ], // -5
            data_used_for: vec!["advertising".into()],
            shared_with_third_parties: true, // -25
            third_party_details: "Ad networks".into(),
            retention_period: "Indefinite".into(),
            user_rights: UserRights {
                can_delete_data: true,
                can_export_data: false, // -10
                can_opt_out_of_tracking: true,
            },
            tracking_and_ads: "Cross-site tracking cookies".into(),
            arbitration_or_class_action_waiver: true, // -30 -> score 30
            policy_clarity_notes: "Vague terms".into(),
        };

        assert_eq!(compute_grade(&summary), 'D');
    }
}
