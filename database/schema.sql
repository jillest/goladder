-- CREATE TABLE seasons (
-- 	id SERIAL PRIMARY KEY,
-- 	name TEXT NOT NULL
-- );
CREATE TABLE players (
	id SERIAL PRIMARY KEY,
	name TEXT UNIQUE NOT NULL,
	initialrating DOUBLE PRECISION NOT NULL,
	currentrating DOUBLE PRECISION NOT NULL
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
	played DATE NOT NULL,
	white INTEGER REFERENCES players (id) NOT NULL,
	black INTEGER REFERENCES players (id) NOT NULL,
	result gameresult -- nullable
);
