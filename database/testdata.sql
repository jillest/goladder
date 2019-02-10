INSERT INTO players (name, initialrating, currentrating) VALUES
	('A', 1000, 1000),
	('B', 1200, 1200),
	('C', 1300, 1300),
	('D', 1350, 1350),
	('E', 1400, 1400),
	('F', 1425, 1425);

INSERT INTO rounds ("date") VALUES ('2019-01-28'), ('2019-02-04'), ('2019-02-11');

INSERT INTO games (played, white, black, result) VALUES
	((SELECT id FROM rounds WHERE "date" = '2019-01-28'), (SELECT id FROM players WHERE name = 'B'), (SELECT id FROM players WHERE name = 'A'), NULL),
	((SELECT id FROM rounds WHERE "date" = '2019-01-28'), (SELECT id FROM players WHERE name = 'D'), (SELECT id FROM players WHERE name = 'C'), NULL);
