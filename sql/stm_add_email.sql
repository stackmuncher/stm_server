-- Inserts an email into t_email_ownership table or updates `is_primary` if exists
CREATE OR REPLACE FUNCTION stm_add_email(_owner_id varchar, _email varchar, _is_primary boolean)
RETURNS void AS $$ --
BEGIN --
--
IF _is_primary THEN
  INSERT INTO t_email_ownership (owner_id, email, is_primary, added_ts) 
  VALUES (_owner_id, _email, now(), now()) on conflict (owner_id, email) do
  UPDATE set is_primary = now() WHERE t_email_ownership.is_primary is null;
ELSE
  INSERT INTO t_email_ownership (owner_id, email, is_primary, added_ts) 
  VALUES (_owner_id, _email, null, now()) on conflict (owner_id, email) do nothing;
END IF;
--
END --
$$ COST 100 VOLATILE LANGUAGE plpgsql SECURITY DEFINER;
GRANT EXECUTE ON FUNCTION stm_add_email(varchar,varchar,boolean) to public;
-- DROP FUNCTION IF EXISTS stm_add_email

/*** TESTING ***/
-- select stm_add_email('9PdHabyyhf4KhHAE1SqdpnbAZEXTHhpkermwfPQcLeFK','max@onebro.me', false)
-- select stm_add_email('9PdHabyyhf4KhHAE1SqdpnbAZEXTHhpkermwfPQcLeFK','test@onebro.me', false)

-- select * from t_owner_idship limit 100 
-- truncate table t_email_ownership
-- select * from t_email_ownership where email = 'max@onebro.me'
-- select * from t_email_ownership where owner_id ='9PdHabyyhf4KhHAE1SqdpnbAZEXTHhpkermwfPQcLeFK'


