from satya.engine.probability import DiePool

from pprint import pprint


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
