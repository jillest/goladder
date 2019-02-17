-- CREATE TABLE seasons (
-- 	id SERIAL PRIMARY KEY,
-- 	name TEXT NOT NULL
-- );
CREATE TABLE players (
	id SERIAL PRIMARY KEY,
	name TEXT UNIQUE NOT NULL,
	defaultschedule BOOLEAN DEFAULT FALSE NOT NULL,
	initialrating DOUBLE PRECISION NOT NULL,
	currentrating DOUBLE PRECISION NOT NULL,
	extra JSONB
);
CREATE TABLE rounds (
	id SERIAL PRIMARY KEY,
	"date" DATE NOT NULL,
	extra JSONB
);
CREATE TABLE presence (
	player INTEGER REFERENCES players (id) NOT NULL,
	"when" INTEGER REFERENCES rounds (id) NOT NULL,
	UNIQUE (player, "when"),
	schedule BOOLEAN NOT NULL
);
CREATE TYPE gameresult AS ENUM (
	'WhiteWins',
	'BlackWins',
	'Jigo',
	'WhiteWinsByDefault',
	'BlackWinsByDefault',
	'BothLose'
);
CREATE TABLE games (
	id SERIAL PRIMARY KEY,
	played INTEGER REFERENCES rounds (id) NOT NULL,
	white INTEGER REFERENCES players (id) NOT NULL,
	black INTEGER REFERENCES players (id) NOT NULL,
	result gameresult, -- nullable
	handicap DOUBLE PRECISION DEFAULT 0.0 NOT NULL,
	boardsize SMALLINT DEFAULT 19 NOT NULL,
	extra JSONB
);
