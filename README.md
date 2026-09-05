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
    - [x] life support: power, O2, water
        x Solar array as module payload slots, should probably start power-positive, but just barely
        x water tanks
        x H2 and O2 tanks, electrolysis module
    - [x] crew: consume O2 and water
    - [x] display time-to-zero (and time-to-fill)
    - [ ] fabricator module: converts feedstock into parts
        x metal as a part
        x recipe data in the toml
        x affordability function, shortfalls()
        x modal shell with cards, read-only
        x build button
        x commits on Next Turn, not build
        x Energy as a continuous draw
        * gui_structure_key includes job state
        * replace build button with progress bar, "Done by ... " text
        * show inventory
    - [ ] assembly bay module: combines parts into spacecraft
    - [ ] can build new modules (or just start with electrolysis module?)
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
    - [ ] Ilmetite mining, smelting (just give generic "metal" for MVP)
    - Rules: 
        * Every resource should have a useful role somewhere in the system, and preferably a secondary use that competes with its first.
        * The byproducts of processes are always useful.
        * Natural resources are clustered out in the system, imperfectly overlap, and distributed based on the hidden parameters.
    - Eventual modules:
        * Electrolysis: H2O + Energy -> H2 + O2
        * Hydrolox Fuel Cell: H2 + O2 -> H2O + Energy (cleaner, but H2 tanks should be a pain)
        * Methalox Fuel Cell: CH4 + O2 -> H2O + CO2 + Energy (not as nice with the CO2, but no H2)
        * Chemistry Lab: Has cartridges for specific processes:
            * Sabatier: CO2 + H2 -> CH4 + H2O
            * Methane Pyrolysis: CH4 -> C, H2
            * CO2 Scrubbing: CaO + CO2 -> CaCO3
            * Haber Bosch: N2 + H2 -> NH3
        * Smelter does a bunch of refinements:
            * TiO2 + Energy -> Ti O2
            * Al2O3 + Energy -> Al + O2
            * SiO2 + Energy -> Si + O2
            * CaCO3 + Energy -> CaO + CO2
        * Greenhouse: CO2 + H2O + Energy -> Food + O2 (composes maybe too well with methalox fuel cell?)
- [ ] Misc stuff
    - [ ] timeline zoom (maybe by dragging the baseline?)
    - [ ] mission planner
    - [ ] allow aero capture when payload has heat shield and player has atmospheric instrument readings
    - [ ] heat management
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