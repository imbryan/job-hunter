use chrono::Utc;
use thirtyfour::By;

use crate::db::{
    job_post::{JobPost, JobPostLocationType},
    NullableSqliteDateTime, SqliteDateTime,
};

#[cfg(target_os = "windows")]
pub const GECKODRIVER_CMD: &str = "geckodriver";
#[cfg(not(target_os = "windows"))]
pub const GECKODRIVER_CMD: &str = "./geckodriver";

pub const GECKODRIVER_PORT: &str = "4444";

pub async fn fetch_job_details(
    driver: thirtyfour::WebDriver,
    url: String,
) -> anyhow::Result<(Option<String>, Option<JobPost>)> {
    if url.contains("linkedin.com") {
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
        let desc_text = desc.text().await?;
        // location type
        let location_type;
        if desc_text.to_lowercase().contains("remote") {
            location_type = JobPostLocationType::Remote;
        } else if desc_text.to_lowercase().contains("hybrid") {
            location_type = JobPostLocationType::Hybrid;
        } else {
            location_type = JobPostLocationType::Onsite;
        }
        // TODO yoe (desc_text)
        // TODO pay (.salary.compensation__salary)
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
                min_yoe: None,
                max_yoe: None,
                min_pay_cents: None,
                max_pay_cents: None,
                date_posted: NullableSqliteDateTime::default(),
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
