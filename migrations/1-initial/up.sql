CREATE TABLE company(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    "name" VARCHAR NOT NULL,
    career_page_base_url VARCHAR
);

CREATE TABLE company_alt_name(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    "name" VARCHAR NOT NULL,
    company_id INTEGER NOT NULL,
    FOREIGN KEY (company_id) REFERENCES company(id)
);

CREATE TABLE job_post(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    "location" VARCHAR NOT NULL,
    location_type VARCHAR NOT NULL,
    "url" VARCHAR NOT NULL,
    min_yoe INTEGER,
    max_yoe INTEGER,
    min_pay_cents INTEGER,
    max_pay_cents INTEGER,
    date_posted INTEGER,
    date_retrieved INTEGER,
    company_id INTEGER NOT NULL,
    FOREIGN KEY (company_id) REFERENCES company(id)
);

CREATE TABLE job_application(
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    "status" VARCHAR NOT NULL,
    date_applied INTEGER,
    date_repsonded INTEGER,
    job_post_id INTEGER NOT NULL,
    FOREIGN KEY (job_post_id) REFERENCES job_post(id)
);
