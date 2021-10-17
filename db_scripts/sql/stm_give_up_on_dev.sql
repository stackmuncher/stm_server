-- Marks the dev as DO NOT RETRY by removing the timestamp of the last submission
-- Only the matching dev is affected.
CREATE OR REPLACE FUNCTION stm_give_up_on_dev(
  _owner_id varchar, _report_in_flight_id uuid) RETURNS void AS $$
BEGIN

UPDATE t_dev
  SET last_submission_ts = NULL, report_in_flight_id = NULL
  WHERE owner_id = _owner_id AND report_in_flight_id = _report_in_flight_id;

END
$$ COST 100 VOLATILE LANGUAGE 'plpgsql' SECURITY DEFINER;
GRANT EXECUTE ON FUNCTION stm_give_up_on_dev(varchar, uuid) to public;
-- DROP FUNCTION IF EXISTS stm_give_up_on_dev

-- TESTING --
-- select * from t_dev limit 100
-- select * from stm_give_up_on_dev('9PdHabyyhf4KhHAE1SqdpnbAZEXTHhpkermwfPQcLeFK','e2b89194-35b1-4d3a-b5e7-fbf2304f84c7')