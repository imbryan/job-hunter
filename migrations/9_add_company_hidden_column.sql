ALTER TABLE company
ADD "hidden" INTEGER;

UPDATE company SET "hidden" = 0 WHERE hidden is NULL;