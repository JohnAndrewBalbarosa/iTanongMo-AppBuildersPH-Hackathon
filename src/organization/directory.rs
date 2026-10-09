use chrono::Utc;
use sqlx::{FromRow, SqlitePool};

use crate::error::{AppError, AppResult};

#[derive(Clone, Debug, FromRow)]
pub struct Agency {
    pub id: String,
    pub uacs_code: Option<String>,
    pub slug: String,
    pub short_name: String,
    pub name: String,
    pub description: String,
    pub official_url: String,
    pub office_email: Option<String>,
    pub source_url: String,
    pub source_as_of: String,
    pub deep_profile: i64,
}

const UACS_SOURCE: &str = "https://uacs.gov.ph/resources/uacs/organization/department";
const DOST_SOURCE: &str =
    "https://www.dost.gov.ph/index.php/transparency/about-dost/organizational-structure";

const DEPARTMENTS: &[(&str, &str, &str, &str)] = &[
    ("04", "dar", "DAR", "Department of Agrarian Reform"),
    ("05", "da", "DA", "Department of Agriculture"),
    ("06", "dbm", "DBM", "Department of Budget and Management"),
    (
        "07",
        "depdev",
        "DEPDev",
        "Department of Economy, Planning, and Development",
    ),
    ("08", "deped", "DepEd", "Department of Education"),
    ("09", "doe", "DOE", "Department of Energy"),
    (
        "10",
        "denr",
        "DENR",
        "Department of Environment and Natural Resources",
    ),
    ("11", "dof", "DOF", "Department of Finance"),
    ("12", "dfa", "DFA", "Department of Foreign Affairs"),
    ("13", "doh", "DOH", "Department of Health"),
    (
        "14",
        "dhsud",
        "DHSUD",
        "Department of Human Settlements and Urban Development",
    ),
    (
        "15",
        "dilg",
        "DILG",
        "Department of the Interior and Local Government",
    ),
    ("16", "doj", "DOJ", "Department of Justice"),
    ("17", "dole", "DOLE", "Department of Labor and Employment"),
    ("18", "dmw", "DMW", "Department of Migrant Workers"),
    ("19", "dnd", "DND", "Department of National Defense"),
    (
        "20",
        "dpwh",
        "DPWH",
        "Department of Public Works and Highways",
    ),
    ("21", "dost", "DOST", "Department of Science and Technology"),
    (
        "22",
        "dswd",
        "DSWD",
        "Department of Social Welfare and Development",
    ),
    ("23", "dot", "DOT", "Department of Tourism"),
    ("24", "dti", "DTI", "Department of Trade and Industry"),
    ("25", "dotr", "DOTr", "Department of Transportation"),
    (
        "26",
        "dict",
        "DICT",
        "Department of Information and Communications Technology",
    ),
];

const DOST_UNITS: &[(&str, Option<&str>, &str, &str)] = &[
    ("dost-osec", None, "office", "Office of the Secretary"),
    (
        "dost-pes",
        Some("dost-osec"),
        "service",
        "Planning and Evaluation Service",
    ),
    (
        "dost-ias",
        Some("dost-osec"),
        "service",
        "Internal Audit Service",
    ),
    (
        "dost-fms",
        Some("dost-osec"),
        "service",
        "Financial and Management Service",
    ),
    (
        "dost-als",
        Some("dost-osec"),
        "service",
        "Administrative and Legal Service",
    ),
    ("dost-pcaarrd", Some("dost-osec"), "council", "PCAARRD"),
    ("dost-pchrd", Some("dost-osec"), "council", "PCHRD"),
    ("dost-pcieerd", Some("dost-osec"), "council", "PCIEERD"),
    (
        "dost-asti",
        Some("dost-osec"),
        "institute",
        "Advanced Science and Technology Institute",
    ),
    (
        "dost-fnri",
        Some("dost-osec"),
        "institute",
        "Food and Nutrition Research Institute",
    ),
    (
        "dost-fprdi",
        Some("dost-osec"),
        "institute",
        "Forest Products Research and Development Institute",
    ),
    (
        "dost-itdi",
        Some("dost-osec"),
        "institute",
        "Industrial Technology Development Institute",
    ),
    (
        "dost-mirdc",
        Some("dost-osec"),
        "institute",
        "Metals Industry Research and Development Center",
    ),
    (
        "dost-pnri",
        Some("dost-osec"),
        "institute",
        "Philippine Nuclear Research Institute",
    ),
    (
        "dost-ptri",
        Some("dost-osec"),
        "institute",
        "Philippine Textile Research Institute",
    ),
    ("dost-pagasa", Some("dost-osec"), "service_agency", "PAGASA"),
    (
        "dost-phivolcs",
        Some("dost-osec"),
        "service_agency",
        "PHIVOLCS",
    ),
    (
        "dost-pshss",
        Some("dost-osec"),
        "service_agency",
        "Philippine Science High School System",
    ),
    (
        "dost-sei",
        Some("dost-osec"),
        "service_agency",
        "Science Education Institute",
    ),
    (
        "dost-stii",
        Some("dost-osec"),
        "service_agency",
        "Science and Technology Information Institute",
    ),
    (
        "dost-tapi",
        Some("dost-osec"),
        "service_agency",
        "Technology Application and Promotion Institute",
    ),
    (
        "dost-regional",
        Some("dost-osec"),
        "regional_group",
        "Regional Offices and Provincial S&T Centers",
    ),
];

pub async fn seed_reference_data(pool: &SqlitePool) -> AppResult<()> {
    let today = Utc::now().date_naive().to_string();
    let mut transaction = pool.begin().await?;
    for (code, slug, short, name) in DEPARTMENTS {
        let id = format!("agency-{slug}");
        let official_url = if *slug == "dost" {
            DOST_SOURCE
        } else {
            UACS_SOURCE
        };
        let email = (*slug == "dost").then_some("foi@dost.gov.ph");
        let description = if *slug == "dost" {
            "Central direction, leadership, and coordination of science and technology activities."
        } else {
            "Executive Department profile. Verify the current FOI receiving office before a production rollout."
        };
        sqlx::query("INSERT OR IGNORE INTO agencies(id,uacs_code,slug,short_name,name,description,official_url,office_email,source_url,source_as_of,deep_profile) VALUES (?,?,?,?,?,?,?,?,?,?,?)")
            .bind(&id).bind(code).bind(slug).bind(short).bind(name).bind(description).bind(official_url).bind(email).bind(UACS_SOURCE).bind(&today).bind(i64::from(*slug == "dost")).execute(&mut *transaction).await?;
        let unit_id = format!("{slug}-receiving");
        sqlx::query("INSERT OR IGNORE INTO organization_units(id,agency_id,parent_id,slug,kind,name,source_url,source_as_of) VALUES (?,?,?,?,?,?,?,?)")
            .bind(&unit_id).bind(&id).bind(Option::<String>::None).bind("foi-receiving-office").bind("foi_office").bind("FOI Receiving Office").bind(UACS_SOURCE).bind(&today).execute(&mut *transaction).await?;
    }
    for (id, parent_id, kind, name) in DOST_UNITS {
        sqlx::query("INSERT OR IGNORE INTO organization_units(id,agency_id,parent_id,slug,kind,name,source_url,source_as_of) VALUES (?,'agency-dost',?,?,?,?,?,?)")
            .bind(id).bind(parent_id).bind(id.trim_start_matches("dost-")).bind(kind).bind(name).bind(DOST_SOURCE).bind(&today).execute(&mut *transaction).await?;
    }
    seed_personas(&mut transaction).await?;
    seed_feature_flags(&mut transaction, &today).await?;
    transaction.commit().await?;
    Ok(())
}

async fn seed_personas(transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>) -> AppResult<()> {
    let personas = [
        (
            "requester-demo",
            None,
            None,
            "Demo Requester",
            "requester",
            "requester",
        ),
        (
            "dost-admin",
            Some("agency-dost"),
            Some("dost-osec"),
            "DOST Agency Admin",
            "agency_admin",
            "staff",
        ),
        (
            "dost-fro",
            Some("agency-dost"),
            Some("dost-receiving"),
            "DOST Receiving Officer",
            "receiving_officer",
            "staff",
        ),
        (
            "dost-decision",
            Some("agency-dost"),
            Some("dost-osec"),
            "DOST Decision Maker",
            "decision_maker",
            "staff",
        ),
        (
            "dost-appeals",
            Some("agency-dost"),
            Some("dost-osec"),
            "DOST Appeals Authority",
            "appeals_authority",
            "staff",
        ),
        (
            "dost-auditor",
            Some("agency-dost"),
            Some("dost-ias"),
            "DOST Read-only Auditor",
            "auditor",
            "staff",
        ),
    ];
    for (id, agency, unit, name, role, actor_type) in personas {
        sqlx::query("INSERT OR IGNORE INTO demo_personas(id,agency_id,organization_unit_id,display_name,role,actor_type) VALUES (?,?,?,?,?,?)")
            .bind(id).bind(agency).bind(unit).bind(name).bind(role).bind(actor_type).execute(&mut **transaction).await?;
    }
    Ok(())
}

async fn seed_feature_flags(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    now: &str,
) -> AppResult<()> {
    for capability in [
        "directory",
        "public_intake",
        "in_app_thread",
        "government_board",
        "deterministic_triage",
        "ai_triage",
        "ai_drafting",
        "mobile_notifications",
    ] {
        sqlx::query("INSERT OR IGNORE INTO feature_flags(capability,agency_id,organization_unit_id,cohort_percent,routing_mode,enabled,updated_at) VALUES (?,NULL,NULL,100,'in_app_primary',1,?)")
            .bind(capability).bind(now).execute(&mut **transaction).await?;
    }
    sqlx::query("INSERT OR IGNORE INTO feature_flags(capability,agency_id,organization_unit_id,cohort_percent,routing_mode,enabled,updated_at) VALUES ('email_bridge',NULL,NULL,0,'email_only',0,?)")
        .bind(now).execute(&mut **transaction).await?;
    Ok(())
}

pub async fn list_agencies(pool: &SqlitePool) -> AppResult<Vec<Agency>> {
    Ok(sqlx::query_as::<_, Agency>("SELECT id,uacs_code,slug,short_name,name,description,official_url,office_email,source_url,source_as_of,deep_profile FROM agencies WHERE active=1 ORDER BY name")
        .fetch_all(pool).await?)
}

pub async fn agency_by_slug(pool: &SqlitePool, slug: &str) -> AppResult<Agency> {
    sqlx::query_as::<_, Agency>("SELECT id,uacs_code,slug,short_name,name,description,official_url,office_email,source_url,source_as_of,deep_profile FROM agencies WHERE slug=? AND active=1")
        .bind(slug).fetch_optional(pool).await?.ok_or(AppError::NotFound)
}
