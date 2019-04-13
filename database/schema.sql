-- CREATE TABLE seasons (
-- 	id INTEGER PRIMARY KEY,
-- 	name TEXT NOT NULL
-- );
CREATE TABLE players (
	id INTEGER PRIMARY KEY,
	name TEXT UNIQUE NOT NULL,
	defaultschedule BOOLEAN DEFAULT 0 NOT NULL,
	initialrating DOUBLE PRECISION NOT NULL,
	currentrating DOUBLE PRECISION NOT NULL,
	extra JSONB
);
CREATE TABLE rounds (
	id INTEGER PRIMARY KEY,
	"date" DATE NOT NULL,
	extra JSONB
);
CREATE TABLE presence (
	player INTEGER REFERENCES players (id) NOT NULL,
	"when" INTEGER REFERENCES rounds (id) NOT NULL,
	schedule BOOLEAN NOT NULL,
	UNIQUE (player, "when")
);

CREATE TABLE games (
	id INTEGER PRIMARY KEY,
	played INTEGER REFERENCES rounds (id) NOT NULL,
	white INTEGER REFERENCES players (id) NOT NULL,
	black INTEGER REFERENCES players (id) NOT NULL,
	result TEXT, -- nullable
	handicap DOUBLE PRECISION DEFAULT 0.0 NOT NULL,
	boardsize SMALLINT DEFAULT 19 NOT NULL,
	extra JSONB
);
