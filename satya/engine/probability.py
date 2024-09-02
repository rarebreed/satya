from dataclasses import dataclass, field
from pprint import pprint
import random as rnd
from typing import Callable


@dataclass
class Die:
    size: int

    def roll(self):
        return rnd.randint(1, self.size)


@dataclass
class DiePool:
    dtype: int
    size: int
    pool: list[Die] = field(init=False)
    explode_on: int | None = None

    def __post_init__(self):
        self.pool = [Die(self.dtype) for _ in range(self.size)]

    def roll(self, pool: list[Die] | None = None):
        if pool is None:
            pool = self.pool[:]

        for d in pool:
            roll = d.roll()
            if self.explode_on is not None and roll >= self.explode_on:
                pool.append(Die(self.dtype))
            yield roll

    def threshold(self, threshold: int,  fn: Callable[[int, int], bool], values: list[int] | None = None):
        if values is None:
            return [r for r in self.roll() if fn(r, threshold)]
        else:
            return [r for r in values if fn(r, threshold)]


def test():
    tests = [(x, y)
             for x in range(4, 11)
             for y in range(1, 20)]
    stats = {}
    for size, th in tests:
        stats[(size, th)] = {}
        stats[(size, th)][0] = 0
        for i in range(1000):
            pool = DiePool(20, size, 19)

            def thresh(rolled: int, thresh: int) -> bool:
                # print(rolled)
                return rolled <= thresh
            successes = pool.threshold(threshold=th, fn=thresh)
            if len(successes) in stats[(size, th)]:
                stats[(size, th)][len(successes)] += 1
            else:
                stats[(size, th)][len(successes)] = 1
        # pprint(stats[(size, th)])
    pprint(stats)
