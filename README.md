# Vis Viva

A turn-based, event-driven space colony survival strategy game. Start off in a sleeper ship sent across the galaxy to a system _beleived_ to have a habitable world, with finite starter resources.

## TODO:
- [x] Procedurally generated solar system
- [x] Delta-V graph
- [x] Stages/payloads
- [x] Dynamic UI elements
- [ ] station
    - [x] station entity with modules/components (start off hovering this! no factory/vab!)
        x station cores have slots for modules, shown in the context menu
        x has builtin batteries
    - [ ] life support: power, O2, water. With time-to-zero ticker prominently displayed
        * Solar array as module payload slots, should probably start power-positive, but just barely
        * water tanks
        * H2 and O2 tanks, electrolysis module
    - [ ] crew: consume O2 and water, die if they run out
    - [ ] fabricator module: converts feedstock into parts
    - [ ] assembly bay module: combines parts into spacecraft
    - [ ] lose the game if the station dies
- [ ] Science
    - [ ] body rotation, axial tilt
        - allows polar mapping probes to actually exist
        * axial tilt affects seasons, temps, could have crazy uranus worlds
        * rotation affects landing and launch delta V
    - [ ] spectrometer, surface comp
    - each tile has hidden parameters: surface temp, atmos temp, surface comp, atmos comp, surface topology, radiation, magnetic field. Ore compositions are derived from these. Some of these are immeasurable from orbit, or only measurable to a noise floor.
    - each instrument has at least one parameter it measures directly, and one or more it provides weaker evidence about indirectly with a noise floor. There should be an H matrix hidden in the game. Totally internal, do the inference for the player.
    - Instruments can be added to certain payloads, which have limited slots. Give insight into what resources _may_ be on the body. Just do the spectrometer for now.
        * IR camera: strong surface temp, mid atmos temp, weak atmospheric composition
        * Radar: strong surface topology, weak surface comp?
        * Spectrometer: strong surface comp, strong atmos comp, weak surface temp
        * Radiation detector: strong radiation, weak magnetic field strength
        * Magnetometer: strong magnetic field strength, weak radiation
- [ ] Mining && ISRU
    - [ ] surface outpots on tiles (give them solar panels for now)
    - [ ] Ice mining, goes into a cargo hold
    - [ ] station rendevous and docking
    - [ ] Ilmetite mining, reduction (just give generic "metal" for MVP)
    - Rules: 
        * Every resource should have a useful role somewhere in the system, and preferably a secondary use that competes with its first.
        * The byproducts of processes are always useful.
        * Natural resources are clustered out in the system, imperfectly overlap, and distributed based on the hidden parameters.
    - Ores (to be scattered on bodies depending on their conditions):
        * Ilmetite => Fe
        * Anorthosite => Al + Si
        * Ice
        * CO2
        * CH4
        * N2
        * Ammonia
        * Uranium
    - Processes (would have dedicated payloads/components for these):
        * Ilmetite reduction: FeTiO3 + H2 + (energy) => Fe + TiO2 + H2O
            - players don't actually get/see TiO2
        * Anorthosite reduction: CaAl2Si2O8 + (energy) => 2Al + 2SiO2 + CaO + 1.5 O2
            - players don't actually get/see CaO
        * Haber-Bosch: N2 + 3H2 => 2NH3 + (heat)
        * Ammonia decomp: 2NH3 + (energy) => N2 + 3H2
        * Sabatier: CO2 + 4H2 => CH4 + 2H2O + (heat)
        * Electrolysis => 2H2O + (energy) => 2H2 + O2
- [ ] food game loop
    - [ ] Plant growth (photosynthesis + respiration + protein): CO2 + NH3 + H2O + (energy) => Food + O2
        - no crop species, no crop rotation, just "food"
- [ ] Misc stuff
    - [ ] timeline zoom (maybe by dragging the baseline?)
    - [ ] mission planner
    - [ ] allow aero capture when payload has heat shield and player has atmospheric instrument readings
    - [ ] atmospheric harvesting
        * get H2 from gas giants, NH3 from ice giants, CO2 from venus-worlds
    - [ ] boiloff for cryo fuels, cooling systems which add weight and take power, tradeoff between cryogenics and hypergolic fuels.
    - [ ] different power sources
        * solar: inverse square proximity to sun => (energy)
        * hydolox fuel cell: H2 + O2 => (energy) + H2O
        * methalox fuel cell: CH4 + 2 O2 => (energy) + C02 + 2 H2O
        * nuclear: U234 => (energy) (heavy but materially efficient)
    - [ ] crew respiration produces small amounts of CO2 that needs to be scrubbed. Later on can be captured.
    - [ ] crew capacity, determine how many simultaneous things can go on in the station.
    - [ ] radiation as a hazard to shield against
        * crew die if they're exposed to radiation, shielding adds mass, mass affects delta V
    - [ ] craft need parachute/landing gear in order to land
    - [ ] win if you beam a message back to earth, huge amount of power, megaproject
- [ ] Misc polish
    - [ ] planetary atmospheres, clouds, tile detail
    - [ ] make actual building models, and rotate them to their tile's normal
    - [ ] show more info about stages, their dv, their resources, maybe a little model sprite, in VAB and factory
    - [ ] show how many stages we have in inventory in the factory