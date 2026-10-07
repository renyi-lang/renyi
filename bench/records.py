"""The same points and products as records.ry, for CPython."""

from dataclasses import dataclass


@dataclass
class Point:
    east: int
    north: int


points = []
for index in range(1, 200001):
    points.append(Point(index, index % 7))
print(sum(point.east * point.north for point in points))
