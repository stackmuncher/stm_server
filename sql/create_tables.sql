-- a collection of all commits and what project they are associated with
DROP TABLE IF EXISTS t_commit_ownership CASCADE;
CREATE TABLE t_commit_ownership (
    -- can be a github login or org (prefixed with `gh:`)
    -- or the public key of the member for inbox submissions, no prefix
    -- e.g. `gh:stackmuncher` or `9PdHabyyhf4KhHAE1SqdpnbAZEXTHhpkermwfPQcLeFK`
  owner_id varchar(200),
  -- can be a github project name or a guid in base58 for private member projects
    -- e.g. `stm` or `Wgx98Rbi8nQuL9ddn3mTk1`
  project_id varchar(150) NOT NULL,
  -- e.g. e29d17e6
  commit_hash varchar(8),
  -- e.g. 1627380297
  commit_ts bigint NOT NULL,

  PRIMARY KEY (owner_id,commit_hash)
);

DROP INDEX IF EXISTS idx_commits_by_owner;
CREATE INDEX idx_commits_by_owner ON t_commit_ownership (commit_hash) INCLUDE (owner_id, project_id, commit_ts);

---------------------------------------------------------------------------------------------------------------

-- A mapping of email addresses to public keys
DROP TABLE IF EXISTS t_email_ownership CASCADE;
CREATE TABLE t_email_ownership (
    -- can only be the public key of the member for inbox submissions, no prefix
    -- e.g. `9PdHabyyhf4KhHAE1SqdpnbAZEXTHhpkermwfPQcLeFK`
  owner_id varchar(200),
  -- the actual email, e.g. max@onebro.me
  email varchar(150),
  -- the timestamp of when the email was first added to the DB, which is either first time it
  -- was encountered in a commit or set as the primary email address
  added_ts timestamptz,
  -- when the email was confirmed to be owned by the owner of the key
  confirmed_ts timestamptz,
  -- an arbitrary sequence of chars used in the confirmation link, contains the timestamp of when it was generated
  confirmation_id varchar,
  -- a ts of when it was set as the primary email for the owner of the key
  is_primary timestamptz,

  PRIMARY KEY (owner_id,email)
);

DROP INDEX IF EXISTS idx_email;
CREATE INDEX idx_email ON t_email_ownership (email);

DROP INDEX IF EXISTS idx_unconfirmed;
CREATE INDEX idx_unconfirmed ON t_email_ownership (confirmation_id);

---------------------------------------------------------------------------------------------------------------

-- contains details of all registered members
DROP TABLE IF EXISTS t_dev CASCADE;
CREATE TABLE t_dev (
      -- can only be the public key of the member for inbox submissions, no prefix
    -- e.g. `9PdHabyyhf4KhHAE1SqdpnbAZEXTHhpkermwfPQcLeFK`
  owner_id varchar PRIMARY KEY,
  -- NULL = no report, date = when the last good report was GENERATED
  -- reset it to NULL to reguest report re-generation
  report_ts timestamp with time zone,
  -- NULL = no active report generation job running, UUID = ID of an active job or the last failed job
  report_in_flight_id uuid,
  -- timestamp when the last report generation job started, may or may not be set to NULL on completion
  report_in_flight_ts timestamp with time zone,
  -- a consequitive number of failures trying to produce a report, reset to 0 on success
  report_fail_counter integer NOT NULL DEFAULT (0),
  -- the timestamp of the latest submission
  -- it should be earlier than the report_ts, otherwise the report is stale and should be regenerated
  last_submission_ts timestamp with time zone
  -- validated github login
  gh_login varchar,
  -- the gist ID used to obtain and validate the gh_login
  gh_login_gist_validation varchar,
  -- the timestamp when the login was validated last time
  gh_login_validation_ts timestamp with time zone,
  -- the gist ID from the latest private submission
  gh_login_gist_latest varchar
);

-- find devs with different combo of report_ts, in_flight and fail counter
DROP INDEX IF EXISTS idx_dev_stale_reports;
CREATE INDEX idx_dev_stale_reports ON t_dev (
  owner_id
)
WHERE (report_ts IS NULL or report_ts < last_submission_ts)  and report_in_flight_id is NULL and last_submission_ts is NOT NULL;