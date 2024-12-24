ALTER TABLE job_post RENAME date_retrieved to date_retrieved_temp;
UPDATE job_post SET date_retrieved_temp = NULL WHERE date_retrieved_temp = 0;

ALTER TABLE job_post 
ADD date_retrieved INTEGER;
UPDATE job_post SET date_retrieved = date_retrieved_temp;

ALTER TABLE job_post DROP date_retrieved_temp;

