-- Inserts multiple commits into t_commit_ownership table
CREATE OR REPLACE FUNCTION stm_add_commits(_owner_id varchar, _project_id varchar, _commit_hash varchar[], _commit_ts timestamp with time zone[])
RETURNS void AS $$ --
BEGIN --
--
INSERT INTO t_commit_ownership (owner_id, project_id, commit_hash, commit_ts)
select _owner_id, _project_id, * from  unnest (_commit_hash, _commit_ts) on conflict do nothing;
--
END --
$$ COST 100 VOLATILE LANGUAGE plpgsql SECURITY DEFINER;
GRANT EXECUTE ON FUNCTION stm_add_commits(varchar,varchar,varchar[],timestamp with time zone[]) to public;
-- DROP FUNCTION IF EXISTS stm_add_commits

/*** TESTING ***/
truncate table t_commit_ownership
select stm_add_commits('o1','p1', array['c1','c2','c3'],array['2021-05-17T04:08:16+00:00','2020-12-25T07:37:31+00:00','2021-04-05T15:29:36+12:00']::timestamp with time zone[])
select stm_add_commits('o1','p2', array['c4','c5','c6'],array['2021-05-17T04:08:16+00:00','2020-12-25T07:37:31+00:00','2021-04-05T15:29:36+12:00']::timestamp with time zone[])
select stm_add_commits('o2','p3', array['c7','c8'],array['2020-12-25T07:37:31+00:00','2021-04-05T15:29:36+12:00']::timestamp with time zone[])
select stm_add_commits('o2','p3', array['c1','c2'],array['2020-12-25T07:37:31+00:00','2021-04-05T15:29:36+12:00']::timestamp with time zone[])

-- select * from t_owner_idship limit 100 
explain analyze select * from t_commit_ownership where commit_hash = any(array['c1', 'c2'])
explain analyze select * from t_commit_ownership where commit_hash in ('c1', 'c2')


