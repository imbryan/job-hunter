use chrono::Utc;
use thirtyfour::By;

use crate::db::{
    job_post::{JobPost, JobPostLocationType},
    NullableSqliteDateTime, SqliteDateTime,
};
use crate::utils::*;

#[cfg(target_os = "windows")]
pub const GECKODRIVER_CMD: &str = "geckodriver";
#[cfg(not(target_os = "windows"))]
pub const GECKODRIVER_CMD: &str = "./geckodriver";

pub const GECKODRIVER_PORT: &str = "4444";

pub async fn fetch_job_details(
    driver: thirtyfour::WebDriver,
    url: String,
) -> anyhow::Result<(Option<String>, Option<JobPost>)> {
    if url.contains("linkedin.com/jobs/view") {
        driver.goto(&url).await?;
        // company name
        let company = driver.find(By::Css(".topcard__flavor a")).await?;
        let company_name = company.text().await?;
        // job title
        let title = driver
            // .find(By::Css(".job-details-jobs-unified-top-card__job-title h1"))
            .find(By::Css(".top-card-layout__title"))
            .await?;
        let title_text = title.text().await?;
        // location
        let location = driver
            .find(By::Css(
                // ".job-details-jobs-unified-top-card__primary-description-container span.tvm__text",
                ".topcard__flavor.topcard__flavor--bullet",
            ))
            .await?;
        let location_text = location.text().await?;

        let desc = driver.find(By::Css(".show-more-less-html__markup")).await?;
        let desc_text = desc.outer_html().await?;
        // location type
        let location_type;
        if desc_text.to_lowercase().contains("remote") {
            location_type = JobPostLocationType::Remote;
        } else if desc_text.to_lowercase().contains("hybrid") {
            location_type = JobPostLocationType::Hybrid;
        } else {
            location_type = JobPostLocationType::Onsite;
        }
        // posted time
        let posted = driver.find(By::Css(".posted-time-ago__text")).await?;
        let posted_text = posted.text().await?;
        let posted_date = NullableSqliteDateTime::from_relative(&posted_text);
        // yoe (desc_text)
        // println!("desc_text {}", &desc_text);
        let (min_yoe, max_yoe) = find_yoe_naive(&desc_text);
        // pay (.salary.compensation__salary)
        let salary = driver.find(By::Css(".salary.compensation__salary")).await;
        let salary_text = match salary {
            Ok(element) => element.text().await?,
            Err(_) => "".to_string(),
        };
        let parsed = parse_salary(&salary_text);
        let max_pay: Option<i64>;
        let min_pay: Option<i64>;
        if let Some((salary, _)) = parsed.get(1) {
            max_pay =
                Some(get_pay_i64(format!("{salary}").as_str()).expect("Failed to get pay i64"));
        } else {
            max_pay = None;
        }
        if let Some((min_salary, _)) = parsed.get(0) {
            min_pay =
                Some(get_pay_i64(format!("{min_salary}").as_str()).expect("Failed to get pay i64"));
        } else {
            min_pay = None;
        }
        // TODO skills (desc_text)
        // TODO benefits (desc_text)
        return Ok((
            Some(company_name),
            Some(JobPost {
                id: -1,
                company_id: -1,
                location: location_text,
                location_type: location_type,
                url: url,
                min_yoe: min_yoe,
                max_yoe: max_yoe,
                min_pay_cents: min_pay,
                max_pay_cents: max_pay,
                date_posted: posted_date,
                date_retrieved: SqliteDateTime(Utc::now()),
                job_title: title_text,
                benefits: None,
                skills: None,
                industry: None,
                pay_unit: None,
                currency: None,
                platform_url: Some("https://linkedin.com".to_string()),
                apijobs_id: None,
                notes: None,
            }),
        ));
    }
    Ok((None, None))
}
