-- Marks the dev as completed by setting report_ts to now()
-- if the in-flight-id matches.
CREATE OR REPLACE FUNCTION stm_complete_dev_job(
  _owner_id varchar, _report_in_flight_id uuid, _gh_login varchar, _gh_login_gist_validation varchar) RETURNS void AS $$
BEGIN

-- update the queue details - happens on every call
UPDATE t_dev
  SET report_ts = now(), report_in_flight_id = NULL
  WHERE owner_id = _owner_id AND report_in_flight_id = _report_in_flight_id;

-- update GH login validation - happens once in a while
UPDATE t_dev
  SET gh_login = _gh_login, gh_login_gist_validation = _gh_login_gist_validation, gh_login_validation_ts = now()
  WHERE owner_id = _owner_id AND (_gh_login is not NULL OR (_gh_login is NULL AND gh_login is NOT NULL));

END;
$$ COST 100 VOLATILE LANGUAGE 'plpgsql' SECURITY DEFINER;
GRANT EXECUTE ON FUNCTION stm_complete_dev_job(varchar, uuid, varchar, varchar) to public;
-- DROP FUNCTION IF EXISTS stm_complete_dev_job

-- TESTING --
-- select * from t_dev limit 100
-- select * from stm_complete_dev_job('9PdHabyyhf4KhHAE1SqdpnbAZEXTHhpkermwfPQcLeFK','e2b89194-35b1-4d3a-b5e7-fbf2304f84c7','rimutaka','fb8fc0f87ee78231f064131022c8154a')

-- update t_dev set gh_login = null, gh_login_gist_validation = null, gh_login_validation_ts = null where owner_id = '9PdHabyyhf4KhHAE1SqdpnbAZEXTHhpkermwfPQcLeFK'
