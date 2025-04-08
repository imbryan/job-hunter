ALTER TABLE company RENAME "hidden" to hidden_temp;
UPDATE company SET hidden_temp = 0 WHERE hidden_temp IS NULL;

ALTER TABLE company
ADD "hidden" INTEGER NOT NULL DEFAULT 0;
UPDATE company SET "hidden" = hidden_temp;

ALTER TABLE company DROP hidden_temp;