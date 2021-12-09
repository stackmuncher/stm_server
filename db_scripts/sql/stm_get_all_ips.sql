-- Returns a unique list of all IPs from t_ip_log
CREATE OR REPLACE FUNCTION stm_get_all_ips() RETURNS SETOF varchar AS $$ -- mark qualifying jobs with the the supplied UUID
  BEGIN --

  RETURN QUERY select ip from t_ip_log;

  END
$$ COST 100 VOLATILE LANGUAGE 'plpgsql' SECURITY DEFINER;
GRANT EXECUTE ON FUNCTION stm_get_all_ips() to public;
-- DROP FUNCTION IF EXISTS stm_get_all_ips

-- TESTING --
-- select * from stm_get_all_ips() limit 100
