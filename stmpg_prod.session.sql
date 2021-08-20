
select * from t_dev

select * from t_email_ownership

update t_dev set report_ts=null, report_in_flight_id=null
where owner_id='9PdHabyyhf4KhHAE1SqdpnbAZEXTHhpkermwfPQcLeFK'

delete from t_commit_ownership where owner_id in (
'B6cHVEU7Wgkn3eo45HnLmPpCDm6Zi7jKcB3qyUiS1fCY',
'Gdt84zRxPfqKLUDRxqxPHLCeDR6wAaCb7g4VDBQUAN9y'

)