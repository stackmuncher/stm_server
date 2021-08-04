-- Inserts multiple commits into t_commit_ownership table
CREATE OR REPLACE FUNCTION stm_add_commits(_owner_id varchar, _project_id varchar, _commit_hash varchar[], _commit_ts bigint[])
RETURNS void AS $$ --
BEGIN --
--
INSERT INTO t_commit_ownership (owner_id, project_id, commit_hash, commit_ts)
select _owner_id, _project_id, * from  unnest (_commit_hash, _commit_ts) on conflict do nothing;
--
END --
$$ COST 100 VOLATILE LANGUAGE plpgsql SECURITY DEFINER;
GRANT EXECUTE ON FUNCTION stm_add_commits(varchar,varchar,varchar[],bigint[]) to public;
-- DROP FUNCTION IF EXISTS stm_add_commits

/*** TESTING ***/
-- select stm_add_commits('o1','p1', array['c1','c2','c3'],array[1627380297,1627338215,1627176058])
-- select stm_add_commits('o1','p2', array['c4','c5','c6'],array[1627380297,1627338215,1627176058])
-- select stm_add_commits('o2','p3', array['c7','c8'],array[1627380297,1627338215])
-- select stm_add_commits('o2','p3', array['c1','c2'],array[1627380297,1627338215])

-- select * from t_owner_idship limit 100 
-- explain analyze select * from t_commit_ownership where commit_hash = any(array['c1', 'c2'])
-- explain analyze select * from t_commit_ownership where commit_hash in ('c1', 'c2')


