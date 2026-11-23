# Thoughts on agent first coding

The role of a software engineer has changed within just the last year thanks to AI agents and harnesses. With the 
onslaught of Gen AI and harnesses like antigravity, kiro-crew, and claude code and others, there has been a 
desire to make the agents as autonomous as possible and only inject HITL (human-in-the-loop) as little as possible.

I will first discuss how to make the agents working on the code create code that is easier for them to digest and
humans to review.  And in the second half, I will point out why there will be chokepoints no matter what, and
whether you want HITL or not.

## Think like an agent

The first step is understanding how reasoning models work and their limitations. AGENTS.md will always get loaded
so they should be lean.  It's more a director of what SKILLS.md to load.  I think of it like when Neo said "I know
kung-fu".  Neo didn't know it until he needed it.  That's a SKILL.  