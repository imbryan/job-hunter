use crate::db::company::Company;
use crate::db::job_post::{JobPost, JobPostLocationType};
use crate::db::{NullableSqliteDateTime, SqliteBoolean, SqliteDateTime};
use crate::job_hunter::utils::format_location;
use chrono::Utc;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::Deserialize;
use serde_json::json;

/* APIJobs.dev */
// https://apijobs.dev/documentation/api/openapi.html //

#[derive(Debug, Deserialize)]
struct APIJobsJob {
    id: String,
    title: String,
    employment_type: Option<String>,
    workplace_type: Option<String>,
    hiring_organization_name: String,
    // hiring_organization_url: Option<String>,
    country: String,
    region: Option<String>,
    city: Option<String>,
    base_salary_currency: Option<String>,
    base_salary_min_value: Option<f64>,
    base_salary_max_value: Option<f64>,
    base_salary_unit: Option<String>,
    experience_requirements_months: Option<i64>,
    skills_requirements: Option<Vec<String>>,
    website: String,
    url: String,
    published_at: String,
}

impl APIJobsJob {
    pub async fn into_job_post(self, executor: &sqlx::SqlitePool) -> JobPost {
        // Get or create company
        let company_id = match Company::fetch_id_by_name(&self.hiring_organization_name, executor)
            .await
            .expect("Failed to fetch company")
        {
            Some(id) => id,
            None => Company {
                id: 0,
                name: self.hiring_organization_name.clone(),
                careers_url: Some(self.website),
                hidden: SqliteBoolean(false),
            }
            .insert(executor)
            .await
            .expect("Failed to insert company"),
        };
        // Handle yoe
        let yoe = self
            .experience_requirements_months
            .map(|months| (months as f64 / 12.0).round() as i64);
        // Handle pay
        let min_pay = self
            .base_salary_min_value
            .map(|dollars| (dollars * 100.0) as i64);
        let max_pay = self
            .base_salary_max_value
            .map(|dollars| (dollars * 100.0) as i64);
        let loc_type = match self.workplace_type {
            Some(loc_type) => {
                let s = loc_type.replace('-', "");
                format!("{}{}", &s[0..1].to_uppercase(), &s[1..])
            }
            None => "Unknown".to_string(),
        };
        let skills = match self.skills_requirements {
            Some(skills_vec) => Some(skills_vec.join(",")),
            None => None,
        };
        let region = match self.region {
            Some(str) => str,
            None => "".to_string(),
        };
        let city = match self.city {
            Some(str) => str,
            None => "".to_string(),
        };
        JobPost {
            id: 0,
            company_id: company_id,
            location: format_location(&city, &region, &self.country),
            location_type: JobPostLocationType::from(loc_type),
            url: self.url,
            min_yoe: yoe,
            max_yoe: None,
            min_pay_cents: min_pay,
            max_pay_cents: max_pay,
            date_posted: NullableSqliteDateTime::from_iso_str(&self.published_at),
            date_retrieved: SqliteDateTime(Utc::now()),
            job_title: self.title,
            benefits: None,
            skills: skills,
            pay_unit: self.base_salary_unit,
            currency: self.base_salary_currency,
            apijobs_id: Some(self.id),
        }
    }
}

#[derive(Debug, Deserialize)]
struct APIJobsJobSearchResponse {
    hits: Vec<APIJobsJob>,
}

pub async fn apijobs_job_search(
    api_key: String,
    companies: String,
    job_title: String,
    location: String,
    min_yoe: i64,
    onsite: bool,
    hybrid: bool,
    remote: bool,
    executor: sqlx::SqlitePool,
) -> anyhow::Result<()> {
    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_static("apikey"),
        HeaderValue::from_str(&api_key).expect("Invalid header value"),
    );
    headers.insert(
        HeaderName::from_static("content-type"),
        HeaderValue::from_static("application/json"),
    );

    let mut loc_types: Vec<&str> = Vec::new();
    if onsite {
        loc_types.push("on-site");
    }
    if hybrid {
        loc_types.push("hybrid");
    }
    if remote {
        loc_types.push("remote");
    }

    let loc_capitalized = {
        location
            .split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                    None => String::new(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    };

    let body = json!({
        "sort_by": "created_at",
        "sort_order": "desc",
        "title": job_title, // "q" is too broad compared to "title" see schema: https://www.apijobs.dev/documentation/api/openapi.html
        "hiring_organization_name": companies,
        // TODO: eventually, location on our side should be processed into subfields
        "country": loc_capitalized, // it REALLY wants countries capitalized
        // "region": location,
        // "city": location,
        "experience_requirements_months": min_yoe * 12,
        "workplace_type": loc_types.join(","),
        "facets": vec!["country", "employment_type", "workplace_type"],
    });

    println!(
        "API REQUEST BODY:\n{}",
        serde_json::to_string_pretty(&body)?
    );

    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.apijobs.dev/v1/job/search")
        .headers(headers)
        .json(&body)
        .send()
        .await?;

    let json = resp.json().await?;
    println!("API RESPONSE:\n{}", serde_json::to_string_pretty(&json)?);

    let parsed: Result<APIJobsJobSearchResponse, _> = serde_json::from_value(json);
    match parsed {
        Ok(parsed) => {
            println!("PARSED API RESPONSE: {:?}", parsed);
            println!("HITS LEN: {}", parsed.hits.len());

            for job in parsed.hits {
                let exists: Option<(i64,)> =
                    sqlx::query_as("SELECT id FROM job_post WHERE apijobs_id = ?")
                        .bind(job.id.clone())
                        .fetch_optional(&executor)
                        .await?;
                if exists.is_none() {
                    let job_post = job.into_job_post(&executor).await;
                    job_post.insert(&executor).await?;
                }
            }
        }
        Err(e) => {
            println!("Failed to deserialize response: {:?}", e);
        }
    }

    Ok(())
}
