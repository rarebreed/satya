# What AI still misses

Depending on who you ask, AI is either about to pop, or it's going to take all the software engineer's jobs away.  I 
think the truth is going to be somewhere in the middle, and the answer also depends on how far out we're talking.

Here's the weird paradox we're in; on one hand, business can't seem to find a ROI on their massive AI spending but on
the other hand we're coding faster than we ever have before.  If we can deliver more features than ever, what's the 
problem then?

- Code Velocity != Customers will buy it
- Generating the code for a feature is just one node in the pipeline
- Goodhart's Law
- Prisoner's Dilemma 

## Code velocity doesn't mean your product is better

This is a really huge assumption that I think misses the forest for the trees.  Is the idea behind more features so that
Company A can say they have more features than Company B and assume that customers will go for Company A?

First off, quanity != quantity.  Secondly, who said it's a feature the customer wants?  Are customers actually asking
for this many new features?  Third, the more features you add, the more the confusing it may be for customers who now
have many more features to choose from.  This also factors into testing, which now experiences a combinatorial explosion
of permutations of features that has to be tested (more on this later)

### The hidden assumption about the code quality

Even just a few months ago, code quality in AI generated programs was suspect.  You never really knew what it might
hallucinate a detail, or run a command that will delete a database.  While those risks still exist, they have been
mitigated.  But I am seeing a different problem with the code that AI generates, and it calls into question the very
idea that _everything_ or almost everything can be automated away by current AI.  I also doubt that this weakness is 
just a matter of a new release by Anthropic or OpenAI.  It's a limitation of transformers in general

What I have observed is that unless you are very careful and precise with your AGENT.md and SKILLS.md files, the agent
or sub-agents will write code that works but follows it to the letter of the law.  

