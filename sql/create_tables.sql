-- a collection of all commits and what project they are associated with
DROP TABLE IF EXISTS t_commit_ownership CASCADE;
CREATE TABLE t_commit_ownership (
    -- can be a github login or org (prefixed with `gh:`)
    -- or the public key of the member for inbox submissions, no prefix
    -- e.g. `gh:stackmuncher` or `9PdHabyyhf4KhHAE1SqdpnbAZEXTHhpkermwfPQcLeFK`
  owner_id varchar,
  -- can be a github project name or a guid in base58 for private member projects
    -- e.g. `stm` or `Wgx98Rbi8nQuL9ddn3mTk1`
  project_id varchar NOT NULL,
  -- e.g. e29d17e6
  commit_hash varchar,
  -- e.g. 1627380297
  commit_ts bigint NOT NULL,

  PRIMARY KEY (owner_id,commit_hash)
);

DROP INDEX IF EXISTS idx_commits_by_owner;
CREATE INDEX idx_commits_by_owner ON t_commit_ownership (commit_hash) INCLUDE (owner_id, project_id, commit_ts)
