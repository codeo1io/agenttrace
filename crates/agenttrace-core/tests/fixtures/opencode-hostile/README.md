# opencode-hostile (rm-753, minted campaign-locally as rm-596)

Hostile `opencode.db` from assess probe P11 of run ff0068ca: five
`session` rows — `s-good` (decodable), a NULL-id ghost, `s-texty`
(TEXT `'not-a-number'` in the INTEGER `time_created`), `s-neg`
(negative token columns) and `s-huge` (i64::MAX tokens). Pre-fix, the
NULL-id row was `filter_map`-dropped and the corpus silently reported
"4 sessions" (sqlite_sessions.rs row loop).

Crafted with python3 sqlite3. Regeneration recipe:

    create table session (id text primary key, title text,
        time_created integer, time_updated integer,
        tokens_input integer, tokens_output integer);
    insert into session values
        ('s-good',  'Good session',      1762000000000, 1762000000001, 100, 200),
        (NULL,      'Ghost session',     1762001000000, 1762001000001, 7, 8),
        ('s-texty', 'Type-abuse session','not-a-number',1762002000001, 1, 2),
        ('s-neg',   'Negative tokens',   1762003000000, 1762003000001, -100, -5),
        ('s-huge',  'Huge tokens',       1762005000000, 1762005000001, 9223372036854775807, 9223372036854775807);

(a crafted sqlite file is never byte-stable across sqlite versions,
hence the sha pin below rather than byte-regeneration).

- opencode.db sha256: 06ead30fd8a1b1e4b5a20a4832d66748e221b976cb4035607cb54c48e3da796a
