-- Inserts a new dev record or updates an existing one for processing
-- when a new report submission is made
CREATE OR REPLACE FUNCTION stm_queue_up_dev_report(_owner_id varchar) RETURNS void AS $$ --
BEGIN --
  -- create a new record if it doesn't exist
  INSERT INTO t_dev (owner_id, last_submission_ts)
  VALUES (_owner_id, now()) on conflict (owner_id) do 
  UPDATE set last_submission_ts = now(), report_fail_counter = 0 
  WHERE t_dev.owner_id = _owner_id;
END --
$$ COST 100 VOLATILE LANGUAGE 'plpgsql' SECURITY DEFINER;
GRANT EXECUTE ON FUNCTION stm_queue_up_dev_report(varchar) to public;
-- DROP FUNCTION IF EXISTS stm_queue_up_dev_report

-- TESTING
-- select * from t_dev limit 100
-- select * from stm_queue_up_dev_report('9PdHabyyhf4KhHAE1SqdpnbAZEXTHhpkermwfPQcLeFK')
