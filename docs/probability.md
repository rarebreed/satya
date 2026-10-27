# How probabilities in satya work

Dice are a common tool to add randomness to some result.  When a task is called for, we want to know whether or not the
action succeeded or failed, and by how much.  Without this randomness, the element of risk and surprise no longer
exists.

Different game systems use different _die mechanics_ to model the randomness that is applied to some task.  There are
many questions that a game's mechanics must address:

- what is the distribution of the random variable (the roll)?
    - what is the variance?
    - what is the mean or expectation?
- What is the distribution of character attributes?
    - what is an average attribute?
- what is an average difficulty?


## Distribution of a random variable (the roll)

You can think of a distribution like a shape of a graph.  A normal or gaussian distribution looks like a bell shaped
curve.  A uniform distribution means that all possibilities are equally likely.  Many distributions have certain
parameters that influence the shape.

For example, normal distributions have a mean, which gives the shape its center, and a variance, which makes the either
steeper or fatter.

