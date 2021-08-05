-- Marks the dev as completed by setting report_ts to now()
-- if the in-flight-id matches.
CREATE OR REPLACE FUNCTION stm_complete_dev_job(
  _owner_id varchar, _report_in_flight_id uuid) RETURNS void AS $$
BEGIN

UPDATE t_dev
  SET report_ts = now(), report_in_flight_id = NULL
  WHERE owner_id = _owner_id AND report_in_flight_id = _report_in_flight_id;

END
$$ COST 100 VOLATILE LANGUAGE 'plpgsql' SECURITY DEFINER;
GRANT EXECUTE ON FUNCTION stm_complete_dev_job(varchar, uuid) to public;
-- DROP FUNCTION IF EXISTS stm_complete_dev_job

-- TESTING --
-- select * from t_dev limit 100
-- select * from stm_complete_dev_job('9PdHabyyhf4KhHAE1SqdpnbAZEXTHhpkermwfPQcLeFK','e2b89194-35b1-4d3a-b5e7-fbf2304f84c7')
