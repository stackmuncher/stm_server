-- Selects a list of devs for processing, marks them with the specified UUID
-- and returns the list to the caller. The number of rows returned is specified
-- by the caller, but the FN imposes a hard limit of 100.
-- Only devs without pending repos are selected.
CREATE OR REPLACE FUNCTION stm_get_dev_jobs(
    _report_in_flight_id uuid,
    _jobs_max integer
  ) RETURNS SETOF t_dev ROWS 100 AS $$ -- mark qualifying jobs with the the supplied UUID
  BEGIN --

RETURN QUERY
WITH d as (select owner_id from t_dev where  (report_ts IS NULL or report_ts < last_submission_ts)  
    and report_in_flight_id is NULL and last_submission_ts is NOT NULL
    FOR UPDATE SKIP LOCKED 
    LIMIT _jobs_max)
  UPDATE t_dev
    SET report_in_flight_ts = now(),
      report_in_flight_id = _report_in_flight_id,
      report_fail_counter = report_fail_counter + 1
    FROM d WHERE t_dev.owner_id = d.owner_id
  RETURNING t_dev.*;

END --
$$ COST 100 VOLATILE LANGUAGE 'plpgsql' SECURITY DEFINER;
GRANT EXECUTE ON FUNCTION stm_get_dev_jobs(uuid, integer) to public;
-- DROP FUNCTION IF EXISTS stm_get_dev_jobs

-- TESTING --
-- select * from t_dev limit 100
-- select * from stm_get_dev_jobs('e2b89194-35b1-4d3a-b5e7-fbf2304f84c7',10)

/* 
explain analyze WITH d as (select owner_id from t_dev where  (report_ts IS NULL or report_ts < last_submission_ts)  
    and report_in_flight_id is NULL and last_submission_ts is NOT NULL
    FOR UPDATE SKIP LOCKED 
    LIMIT 10)
  UPDATE t_dev
    SET report_in_flight_ts = now(),
      report_in_flight_id = 'e2b89194-35b1-4d3a-b5e7-fbf2304f84c7',
      report_fail_counter = report_fail_counter + 1
    FROM d WHERE t_dev.owner_id = d.owner_id
  RETURNING *;
*/




