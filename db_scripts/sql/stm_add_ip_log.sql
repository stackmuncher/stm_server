-- Inserts or updates an IP address log
CREATE OR REPLACE FUNCTION stm_add_ip_log
(_ip varchar[], _cnt bigint[], _added_ts timestamptz[], _latest_ts timestamptz[])
RETURNS void AS $$
  BEGIN

    INSERT INTO t_ip_log (ip, cnt, added_ts, latest_ts)
      values (unnest(_ip), unnest(_cnt), unnest(_added_ts), unnest(_latest_ts))
      on conflict (ip) do update set cnt = t_ip_log.cnt + excluded.cnt, latest_ts = excluded.latest_ts;

  END --
$$ COST 100 VOLATILE LANGUAGE plpgsql SECURITY DEFINER;
GRANT EXECUTE ON FUNCTION stm_add_ip_log(varchar[], bigint[], timestamptz[], timestamptz[]) to public;
-- DROP FUNCTION IF EXISTS stm_add_ip_log

/*** TESTING ***/
-- select stm_add_ip_log(array['0.0.0.0']::varchar[],array[10]::bigint[], array['2021-01-01 00:00:00']::timestamptz[], array['2021-01-02 00:00:00']::timestamptz[])
-- select stm_add_ip_log(array['0.0.0.0','0.0.0.1']::varchar[],array[10,5]::bigint[], array['2021-01-01 00:00:00','2021-01-02 00:00:00']::timestamptz[], array['2021-02-01 00:00:00','2021-03-02 00:00:00']::timestamptz[])
-- select * from t_ip_log
-- truncate table t_ip_log