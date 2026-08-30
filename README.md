# Delta Horizon

A turn-based, event-driven space program strategy game. 

## TODO:
- [x] Procedurally generated solar system
- [x] Delta-V graph
- [ ] Stages/payloads
    - [ ] solid rockets should fire for their entire duration (making them very imprecise!)
- [ ] Economy
    - [ ] starter-world resource market, that sells materials at doggy doodoo prices compared to what you could extract from ISRU
    - [ ] quarterly funding scales with prestige, shrinks if player goes quiet
- [ ] Tech tree
    - [ ] TechTree which tracks the conditions and what's unlocked
    - [ ] start off with really bad solid stage parts, have to progress towards better ones
        * first sateilite in orbit => unlocks better launch stages
        * first probe flyby of moon => unlocks better upper stages
        * first probe orbit around other body => unlocks probe lander
        * first probe landing on other body => unlocks human lander
        * first craft in solar orbit => unlocks better transfer stages
        * discover water ice somewhere => unlocks better hydrolox stages
        * discover methane somewhere => unlocks better methane stages
        * human landing and returning from another body => base parts? station parts?
- [ ] timeline at the bottom which displays upcoming events, allows user to see, plan, and subscribe to transfer windows
- [ ] Science
    - Instruments can be added to certain payloads, which have limited slots. Give insight into what resources _may_ be on the body
        * Imaging
        * Spectrometer
        * Mass Spectrometer
        * Magnetometer
        * Radiation
        * Radar
        * Thermal mapping
        * Gravimeter
        * Seismometer
        * Micrometerorite
- [ ] Mining && ISRU
    - Ores (to be scattered on bodies depending on their conditions):
        * Ilmetite => Fe
        * Anorthosite => Al
        * Pyroxene, orthoclase? => Si
        * Ice
        * CO2
        * CH4
        * N2
        * Ammonia
    - Processes (would have dedicated payloads/components for these):
        * ??? some kind of reduction?: Rock ore => Metal
        * Sabatier: CO2 + H2 => CH4 + H2O
        * Electrolysis => H2O + (energy) => H2, O
    - Factories require metal and fuel, can either buy it from the market on earth, or find it in solar system
    - Bases and stations require continuous power, water, food in order to work, makes ISRU necessary
    - ISRU equipment should be heavy, slow, and body-specific
- [ ] Bases & stations
    - require power, oxygen, food, water
- [ ] Misc polish
    - [ ] figure out how to fix turn based-ness
        * if only next event is payday, 3 whole months go by that might have been a window
        * or if a player has 12 craft up, they're constantly hitting getting interrupted
        * probably just need more play testing to really find an answer
    - [ ] Ability to buy new tiles to build new factories
    - [ ] Dynamic UI elements
        * progress bar going up as turn animates
        * calendar string changing as turn animates
        * scrollable containers (gonna be super imporant!)
    - [ ] body rotation (could help with taking off)
    - [ ] allow aero capture when payload has heat shield and user has atmospheric instrument readings
    - [ ] crew + life support
    - [ ] power (solar, nuclear, fuel cell, baterry) + heating + cooling components in payloads
        * craft die without power, over/under heat
    - [ ] radiation zones
        * craft fail if not shielded
    - [ ] comm range limits, line-of-sight (to incentivize comm networks and relays)
    - [ ] re-usable first stages that return back to the bodys inventory after launch
    - [ ] tug payloads that transport fuel, raw materials or other craft
    - [ ] planetary atmospheres
    - [ ] asteroids
    - [ ] Space stations, bases
    - [ ] make actual building models, and rotate them to their tile's normal
    - [ ] show more info about stages, their dv, their resources, maybe a little model sprite, in VAB and factory
    - [ ] show how many stages we have in inventory in the factory