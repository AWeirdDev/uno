# Players
{n_players}

You are **player {player_id}**.

# Your cards
ID - CARD
{cards}

First, analyze what card you'd like to go with, then **put the index of the card to use** in the **very last line of your response**.
In other words, **only a number is expected in the last line of your response**, with no other text descriptions.
**HOWEVER**, if you have no cards to go with, just **type "draw" in the last line**.
**NOTE**: Card IDs are **zero-indexed**.

# Response Format (you have a card)
For instance, if you want to put the card with index 0, respond with:

... (your thoughts here)
0

...this **does not** mean the number 0, but rather **the card pointed to the index 0**.

# Response format (you have no card)
If you have no card to go with, respond with:

... (your thoughts here)
draw

No matter what, the last line of your response must have either a number or the word "draw."