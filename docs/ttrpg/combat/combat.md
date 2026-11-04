# Combat 

Combat is a highly detailed affair, and in general there are two forms of combat: ranged and melee.  Both have some
common elements which is what will be discussed in this document:

- Action economy
    - Effect of fatigue, wounds, experience and psychological factors on OODA
    - Create list of Time Cost for common actions
    - Determine Time Factor (TF * TC = Action Count Cost)
- Status tracking
    - Fatigue
    - Wounds including pain
    - Physical Effects
        - Blood loss and hemorraghing
            - Shock
        - Structural (by location)
            - Amputation
            - Broken or crushed
            - Sprains, tears, dislocations (if applicable)

## The action economy

Determining what a character will do, and when the action takes effect is perhaps the most crucial factor in detemining
combat.

# Combat in satya

Because I have always been fond of simulation style games, combat is a large part of the design of satya.  Combat is
really just a specialized and detailed type of Task Resolution.  As in all Task Resolutions, it is a contested system.
Even ranged combat is a contested roll.

But what really separates combat in satya is the detail and level of choice in combat compared to other systems. 
Almost everything about combat in satya is different compared to mainstream RPG's.  I know this system will not be for 
everyone, but for those who are into detail, crunch and simulation, I hope this will feel right at home.

## Aspects of combat

So what makes satya different?

Most games only factor in skill of the adversaries, how fast they are, weapon damage and armor. Most games typically
do combat along these lines:

- Determine who has initiative, so that their actions go first or can preempt others
- Choose an action (some kind of attack or feat)
- One being attacked can determine some kind of active or passive defense if possible
- Determine odds of attacker hitting
- If hit, roll for damage, possibly rolling for hit location and subtract armor if any from damage
- The character with the next initiative repeats the above, rinse, later, repeat

Most games don't consider (or bolt them on in a very ad-hoc manner via feats or advantages) any of the following:

- Morale: If you are scared, you may not even get to choose your action or be at combat disadvantage
- Aggressiveness: "It's not the size of the dog in the fight, it's the size of the fight in the dog"
- Fatigue: It doesn't matter how skilled you are if you are too tired to move
- Damage: Wounds, concussions, broken limbs, pain and other damage effects can drastically reduce effectiveness
- Reach: If your weapon can't reach your target, you have to get into range (which can be difficult and takes time)
- Time: How fast does your (re)action take?  It's not just when an action starts that matters, but how soon it finishes
- Reliability: Especially for modern weapons, how prone is your weapon to failure?

Some will argue that factoring in the above is too much minutiae to keep track of, and will bog down the "fun" of the
game.  I will argue that without consideration of these elements, the narrative value of the action is lost, and perhaps
worse, the unrealism will tar victories with a poor brush.  Some will argue that many systems _do_ address many of the
above aspects, but I feel when they are factored in, they are handled in an indirect ad-hoc way with special rules
bolted on (think Feats or Advantages in many systems) and therefore are handled in an incongruous way.  If it is done
smartly and consistently, I believe that all the above can be factored in with the added benfits:

- make combat much more visceral and even narratively exciting by having more tactical choices
- the realism will make victory all the more sweet
- the added complexity will be compensated for by consistent rules
- while each combat "turn" will be longer, time spent in combat overall will be the same due to it being more deadly
- will make characters think twice about combat to resolve things

## Genre support

satya is not a super-hero fantasy type system at its heart.  While it could model such things, satya is geared towards
modelling real-world human level skills.  Moreover, player characters do not have any inherent advantages over NPCs.
They do not get to dodge bullets, and most solid gun shots (a decent margin of success) will incapacitate a character no
matter how much experience and training they have.

The idea behind the realism is two fold:

- Make combat interesting by providing more tactical choices
- Make combat more memorable and conscious by making it more deadly

Too many game systems that are not war related make combat a first choice option.  This is because in most systems, the
characters can take multiple solid hits until they are hors-de-combat, and helpful healing spells or sci-fi med kits can
instantaneously heal at least some of the damage done.  Also, many game systems don't take into account how being
wounded or fatigued affects performance.  In many game systems as long as your "hit points" is not 0 or less, your
character is functioning perfectly normal  This is not only unrealistic from a simulation point of view, it is boring
from a story telling point of view.  How many stories are made more dramatic and memorable because the protagonist has
to overcome pain, fatigue, or physical constraints imposed by wounds?  So I as the game author emphatically assert that
hit point style systems without some kind of wounding effect are not just unrealistic and thus lead to one dimensional
play, but poor at story telling as well.

In other words, I want to dispell the myths that

- realism is too time consuming
- realism isn't fun or interesting because it doesn't let you do cool things

## Design philosophy of combat in satya

Most combat in RPG's are not realistic or even narratively interesting at all.  Most games typically allow each
participant in the combat an equal number of actions and reactions.  And even games that let characters have more
actions than others still allow automatic defensive reactions, and generally apply an evenness or "fairness" to combat.
Combat however, is anything but fair, and if you are playing fair, either you are in a duel, or doing it wrong.  Most
games show this fairness by trading action for action:

```
Character A attacks
NPC B blocks/dodges
NPC B attacks
Character A blocks/didges
```

This is not only unrealistic, it is narratively boring.  How many boxing matches or UFC fights do you see alternating
like that? None.  In the real world, opponents size each other up then unleash a flurry of attacks, stopping only to
rest, regain balance, or because the defender was able to counter and seize the initiative.  For example, the defender
might be trapping the attack, sweeping the legs, or side stepping out of the line of attack.  Or maybe one combatant
only throws out an exploratory jab or thrust, perhaps as a feint, or perhaps to judge his opponents reflexes, and the
other combatant may still just hold, waiting for an opening to attack or escape.  

In the latter scenario where a defender only defends, what is in it for him game-wise in most systems?  All he has done
is allow the attacker a shot.  Without rules for fatigue or tactical advantage, there is absolutely no reason from a
meta game perspective for a character to ever hold back attacks.  Perhaps in other game systems, there is a special rule
(ala a Feat or Advantage) where if they defend for one round, they gain some kind of tactical advantage. But then you
have to learn the umpteen million special Feat rules, and how they interact with each other in combat.  I will argue
that although satya initially looks more complex, in the long run it is actually simpler, because there are less special
rules you need to learn or keep track of.

Going back to the example fight, many fans of other game systems will simply tell you that all of the above _is_
happening in those games, but the turn and initiative system simply abstracts it away.  In other words, a roll of the
dice doesn't reflect a _single_ blow or a single defense.  Instead, the rolls are an abstraction of one or more attacks
or defenses, and the result of the dice is the sum effect of the actions.  And yet, how often is the narrative described
that way?  Also, even if the above is true, why does each side effectively always get to attack and defend?  And why do
the opponents each get the same number of chances to attack/defend?  That is not how real fights work, as when you hear
in boxing "he's got him on the ropes", "he's on his last legs", "the enemy has seized the initiative", "the enemy is on
the run", etc etc.  So I deny that even if you assume combat is abstracted into multiple blows, feints, parries, etc, it
still allows for "fairness" by allowing each opponent similar opportunities.

The way virtually every combat system works is the combatants stand their ground, and trade blow for blow until someone
gets a good shot in (ie, a good dice roll).  If you argue that the combat is abstracted away, and the combatants are
jockeying for advantageous position, sizing each other up, stepping back to take a breath etc, how often is that
actually described?  I argue that you have just abstracted not only alot of the fun away (tactical decision making) but
also much of the storytelling, narrative description, and the reward of victory (by making good choices, rather than
letting the dice abstract it away).  Wouldn't you rather actually make those distinctions _in the game rules_ rather
than (forget) to make them up as flowery description?

The most common argument against such crunchy rules is that it makes combat take longer and thus takes away from the
story by focusing too much time on combat.  Though combat will take more time due to this level of simulation, I feel it
is a worthwhile trade.  As I mentioned earlier, I think it is actually better for storytelling than the too abstracted
combat systems of other games.  I also will argue that victory will feel more sweet, because you have put more thought
and effort into it.  I will also argue that combat may not take as much time as other systems, because combat in satya
will be more lethal or at least render characters hors de combat much faster than most games.  While simulating the
action may take longer, successful hits will be more brutal than most games (especially ones using hit points) and thus
combatants will be taken out of combat with fewer hits (often just one).

> Side rant: hit points
>
> The popularity of that other game has made the notion of hit points ubiquitous. It also strains versimilitude to the
> breaking point when you have an assassination, coup de grace, or some other situation where a high hit point character
> should be killed very easily.  The old tired excuse is that hit points reflect skill and fortitude of more experienced
> characters, who are therefore able to turn a mighty blow into, essentially a scratch.  But over time, due to fatigue
> and "luck running out", eventually even very experienced characters will eventually succumb to their wounds.
> 
> This is horse crock.  Why then are there rules about number of hit points healed per day?  So it takes longer for a 
> character with high hit points to be healed because....it takes longer to heal their luck?  It's an abstraction so 
> leaky, that a little Dutch boy would feel at home.  But, because it has such a long hold in the industry, most players 
> dont give it a second thought.  And it's not just bad from a realism point of view, it's bad from a gaming point of view
> due not just to the credibility strain, but because it fosters a cavalier attitude towards combat.  There's a reason
> that some games require your opponents to "level up" with your characters in order to make combat feel more dangerous.
> Secondly, from a gaming point of view, it can make combat take longer, because characters have more ability to soak up
> damage from "lesser" opponents.
> 
> Some game systems have moved away from Hit Points, or at least a single "bag" of HP.  In satya, HP is replaced the 
> notion of damage tracks, with different kinds of damage.  It also pays a lot of attention to conditions and affects of
> damage types.


## Combat factors

Considerations of fatigue, distancing, aggressiveness, morale, psychology and looking for openings is almost never
considered in games.  satya is different, and many subtleties of combat are considered.  Most games dont factor in:

- What is the attack's reach?
- Distancing between opponents
- Facing or positioning of opponents
- How fatiguing is the maneuver?
- How fast will the attack hit (not who started first)?
- How to take advantage of or create openings
- How to take advantage of an opponent who telegraphs
- Simultaneous attacks
- Stability of an attack or defensive movement
- Motivation to fight
- Emotional state

## Example of a melee fight

Before going too deep, let's give an example of a fight

1. Determine psychological state
2. Determine awareness (surprise)
3. Determine distance to contact
4. Plan maneuver in secret
    - Allocate pool to triad: power, movement/speed, accuracy
5. Make combat awareness check to determine when opponent's move will be realized
6. Make a skill and psychology check to determine what to do based on opponent
    - Factor in any reaction time change if action is changed

## Psychological state of combat

Combat is greatly affected by the psychological state of the fighters.  The fighter's mind and will to fight will often
be defeated before the body.

In order to model the psychological state of combat for a character we have the following ratings:

- Fight: how willing you are to do harm
- Preservation: how needful you are to stay alive
- Rage: 
- Fear

### Motivation

Fighters who have nothing to lose or are in a vengeful mood will be less affected by morale than fighters who don't know
why they are fighting or have nothing to gain.  If you are fighting to save your home country, or acting as a fighting 
rear guard to allow your comrades to escape, you will be much

**TODO** What are the game mechanics for this?

Motivation provides a defense against morale loss. Individually, or as a group, if morale drops, then the will to fight
drops to 0, and no offensive moves can be taken, only moves for defense. If morale drops to 0, then the character or
unit has only self-preservation in mind.

### Mental state

Some people are just more aggressive or violent than others.  Others are just more scrappy, and have a love of fighting.
As Churchill famously said, "It's not the size of the dog in the fight, but the size of the fight in the dog".
Conversely, some people are timid, meek, or even cowardly, and either have no heart to harm others, are scared, or too
afraid of harm being done to themselves.

Essentially, aggressiveness is the ratio of desire to do harm to others, vs willingness to be harmed.  The higher this
ratio is, the more you want to "dish it out".  The lower this ratio, the more defensive you become.  Notice the use of
the words _desire_ and _willing_.  A character may be very willing and able to do harm to other NPCs, but may not have a
particular _desire_ to do so.  A berserker can have a high aggression, where an assassin, who is more than willing or
capable to kill someone, won't be aggressive unless there is a purpose or job for example.

Aggressiveness is rated as Harm Other/Self Protection.  If you have a low Protect:Self, then it means you are more
willing to sacrifice yourself or commit harm upon yourself to achieve some goal.  Think for example of suicide bombers
or kamikaze pilots. 

But how do we determine this score, and does it change over time?

#### How the balance changes

This is where a Character's psyche and personality traits come into play.  

Aggression will have a starting value based on two factors:

```
Personality and Situation
```

Some people are naturally more aggressive than others and this will be indicated as part of their personality.  Note
that aggression isn't just physical.  A highly aggressive person is also quick to launch insults or be emotionally
aggressive as well.  It all comes as a package.  Situation is context dependent and so must be judged on the spot.

Situational modifiers include things like hatred of another race, political group, religion or other faction.  It can
also be lessened as well, like if your character has a sense of honor, and the opponent is somehow at a disadvantage.
Seeing suffering can also sometimes lessen aggression.  What can trigger these is left to GM discretion, but should
take into account the Psyche and Personality traits.

For example, Rage can drastically increase both Harm:Other and lower Self:Protect simultaneously.  Selfishness or
cowardly traits can also dramatically raise Self:Protect and reduce Harm:Other.  Being wounded typically puts one into
survival mode, thus increasing Self:Protect.

Training, discipline and personality traits can affect when and how much these values change.

| Trigger              | Check  
|----------------------|--------
| Being wounded        |
| Good friend wounded  | 
| Loved one attacked   |
| Loved one wounded    |
| Insulted             |
| Humiliated           |
| Seeing suffering     |


**TODO** What are the game mechanics for this?

Aggressiveness comes with some pros and cons.

#### Effects of aggression

### Morale

We all would like to think our character would bravely face down a horde of orcs, enemy soldiers or whatever your
nemesis happens to be.  But the fact is, having a character essentially be immune to fear cheapens the very heroism
they are supposed to have.  Nothing is valuable without sacrifice...and this includes having to pay for overcoming one's
fears.

Many factors can play into morale.  A substantial part of morale comes from:

- Psyche: your personality traits (especially egocentricity)
- Principles: how you believe you should live your life
- Training: A sense of duty and obligation, or cultural way of life

If your homeland or town is being razed to the ground, it can throw thoughts of one's own death to the winds if you are
the type that is patriotic or humanitarian.  If your character is the self-centered type (a very high ego) your
character may feel that everyone else is unimportant except one's own skin.

Morale is lowered under several circumstances:

| Trigger                      | Effect
|------------------------------|------------
| 25% of your party is down    | -1
| 50% of your party is down    | -2
| 75% of your party is down    | -3
| outnumbered 2 to 1           | -1
| outnumbered 3 to 1           | -2  
| outnumbered > 4 to 1         | -3
| leader is unreachable        | -1
| headquarters is overrun      | -3
| (Modern) down to 20% ammo    | -2
| attacks are doing nothing    | -3
| start out fatigued           | -1

### Base morale

Morale is determined by:

**TODO: Morale is like a state that changes over time, but what is the base?  An innate characteristic?**

### Conditions of morale

There are 5 conditions of morale

- High: You and your team is eager and ready to attack.  Advantage to aggression
- Steady: The default condition.  Your character is ready and willing to fight and be agressive.
- Wavering: You are holding and can still attack, but penalty to aggression and offensive attacks
- Broken: You can no longer attack and must withdraw.  Can perform covering fire/supression only
- Panic: Full blown route, your only action is to turn your back and flee

When you have **High** morale, your character is chomping at the bit to fight.  There is a +1 bonus for aggressive
actions (taking the fight to the enemy to try to put them off guard)

For **Steady** morale, your character is in the fight and equally capable of both offensive and defensive techniques.

At **Wavering** morale, your character is starting to wonder if they are going to lose.  There is a -1 bonus for any
offensive techniques, and the character switches to a defensive mode.

At **Broken** morale, your character will try to disengage from combat in the most effective way possible. No actively
offensive techniques are possible.  If attacked physically, the character can perform passively offensive maneuvers
(counter blocks, trips, disarms, throws, etc), but at a -2 rating.  Dodges and other maneuver-to-position moves are
possible without any ill-effects.

When in **Panic**, the character must flee at the maximum speed possible.  This even means turning one's back to the
enemy!  A roll can be made here to surrender to see if the character realizes that is hopeless (eg the enemy can move
faster than he can).

### Rolling for morale

**TODO** how does game mechanics work for morale?

## Melee combat

Melee combat is different from ranged combat in many ways.  The in your face nature of melee combat is not only
different in terms of tactics and training, but also psychologically.  This section goes into detail about how to run
melee combats

### Tactical choices: GRAMS

What makes satya somewhat unique is that it revolves around a tactical pool that must be split among different
values.  Pick 3 out of 5:

- Guard: How open are you leaving yourself?
- Root: How stable vs mobile are you? What is your stance (high means rooted and less mobile)
- Accuracy: How precise is your maneuver?
- Magnitude: How much force are you putting behind the attack or distance to your dodge/evasion?
- Speed: How fast is your maneuver?

Remember this as GRAMS

### Guard

Guard is a measure of how open or vulnerable you leave yourself to a counterattack.  Fighters who go all out, by 
definition practically, must expose themselves in order to do an all out attack.  Reaching excessively, as in a lunge,
or winding up to deliver a powerful blow often leaves the attacker exposed.

A high rating means that the attacker gets a defensive bonus against counter attacks.

**TODO** Flesh out the game mechanics

### Root

Many martial arts stress being rooted.  This is a measure of how hard it is to throw you off balance.  The higher your
root, the harder it becomes to be thrown, tripped, or knocked off balance.  Any movement required during an attack will
deduct from this score.  So for example, if you need to close in to range, the movement required will be subtracted.

It is difficult therefore to be both mobile, and rooted.  On one hand, you gain a defensive bonus to being tripped,
thrown, or grappled, but the score here is also a penalty against evasion technique (ie, evading a grenade)

### Accuracy

Some strikes are wild while others require a more targeted area.  Many locks for example, require very precise location
in order to work correctly.  The rating here can be used to offset any targeting penalties.

From a defensive standpoint, accuracy implies blocking, parrying, or manuevering to a desired position (dodging for a 
tactical advantage).  Adding accuracy to a positioning manuever, will for example become a bonus for the next attack.

### Magnitude

How much force is put into the attack, block, or movement for an evasion.  Attacks can be light (think about a jab) or
punishing (think of a haymaker).  The same goes defensively.  Defending a heavy attack by a 2 handed weapon either 
requires a strong block also, or finesse with a parry.  Dodges may also require greater magnitude representing a larger
distance moved to evade (imagine dodging a spear for example or a whip chain)

### Speed

How quickly you pull off a maneuver is determined by how much speed you put behind it.  The higher this rating, the 
less Phases it takes to complete and also the harder it becomes to block or dodge.

From a defensive viewpoint, a quick block is needed to defend a quick attack.

## Techniques

Every melee combat is defined by a techniques.  Techniques can be offensive, defensive, or both in nature. Many
defensive moves can actually be considered counter attacks.  For example, blocks, traps, or disarms can be considered
defensive moves which can damage the attacker.  A defense is therefore a reactionary technique, whereas an offensive
move is an active technique.  So a move which may seem defensive, like a manuever (which can double as an evade) is
actually an offensive technique, and not defensive.

| Offense     |
|-------------|---------------------|
| strike      |
| grab        |
| throw       |
| 

Offense:

- SAM: Hitting hard, fast and accurate, but poorly rooted and guarded (think lunging punch, or flying kick)
- SAG: Fast accurate guarded strike, but not rooted or powerful (quick snap quick)
- SAR: Fast accurate and rooted, but not powerful or guarded (flurry of jabs)
- SMG: Fast powerful and guarded, but not accurate or rooted (flying knee or spinning kick)
- SMR: Fast powerful and rooted, but open wide and inaccurate (hook, roundhouse)
- SGR: Fast, guarded, rooted, but not powerful or accurate  (quick weaving jab)
- AMG: Accurate, powerful and guarded, but not fast or rooted (knee kick)
- AMR: Accurate powerful and stable, but not fast or guarded (haymaker)
- AGR: 
- MGR:

Defense:

- SAM: quick leap out of the way to maneuver beneficially
- SAG: Fast accurate stable strike, but not rooted or powerful (quick snap quick)
- SAR: Fast accurate and rooted, but not powerful or guarded (flurry of jabs)
- SMG: Fast powerful and guarded, but not accurate or rooted (flying knee or spinning kick)
- SMR: Fast powerful and rooted, but open wide and inaccurate (hook, roundhouse)
- SGR: Fast, guarded, rooted, but not powerful or accurate  (quick weaving jab)
- AMG: Accurate, powerful and guarded, but not fast or rooted (knee kick)
- AMR: Accurate powerful and stable, but not fast or guarded (haymaker)
- AGR: 
- MGR:

### Combat mode

A combatant will start with a mode.

- Disengaging: Trying to flee combat
- Defensive: Concentrating on defending and looking for openings
- Opportunistic: 
- Offensive
- Aggressive

## Ranged combat

Ranged:

- Speed (aim time)
- Stance
- Awareness