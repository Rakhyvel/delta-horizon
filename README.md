# Vis Viva

A turn-based, event-driven space colony survival strategy game.

## TODO:
- [x] Procedurally generated solar system
- [x] Delta-V graph
- [x] Stages/payloads
- [ ] Dynamic UI elements
    x progress bar going up as turn animates
    x calendar string changing as turn animates
    * timeline at the bottom which displays upcoming events like transfers and depletion dates, allows user to see, plan, and subscribe to transfer windows
    * scrollable containers (gonna be super imporant!)
    * better part cards
- [ ] station
    - [ ] station entity with modules/components (start off hovering this! no factory/vab!)
    - [ ] life support: power, O2, water, food. With time-to-zero ticker prominently displayed
        * Solar panels and batteries as module payload slots, should probably start power-positive, but just barely
        * O2, H2 tank modules, and a water converter? Or would it be more likely to store the water as a liquid/solid, and use electrolysis to separate it out?
        * C02 scrubber
        * small aquaponics module, food stores
    - [ ] crew population, can assign roles, or send back to "waiting assignment"
        * consume O2, water, food. When they're at stations, the station likely consumes power
        * can unlock automation to free up the crew for other stuff
    - [ ] fabricator module: small version of the factory, part-size/complexity limited. Upgradable
    - [ ] assembly bay module: like the VAB, craft-size limited. Upgradable
    - [ ] low gain comms module (S band?), high gain comms module (X band?), unimplemented for now
    - [ ] radiator module, unimplemented for now
    - [ ] lose the game if the station dies
    - [ ] win the game if you can beam a message back to earth?
- [ ] body rotation, axial tilt
- [ ] terrain displacement
- [ ] Science
    - each tile has hidden parameters: ore content, global+local temp, radiation, etc. The instruments below refine the estimates of the parameters. Don't make the players do the actual math or whatever, just show them overlays for "ilmetite: belief +- margin"
    - Instruments can be added to certain payloads, which have limited slots. Give insight into what resources _may_ be on the body. Just do orbital imaging and spectrometer for now.
        * Imaging: ore, atmosphere presence
        * Spectrometer: ore, atmosphere makeup
        * Mass Spectrometer: ore, atmosphere makeup
        * Magnetometer: ore, radiation, atmosphere presence
        * Radiation: radiation... duh
        * Radar: surface hazards (volcanoes, mountains)
        * Thermal mapping: temperature
        * Gravimeter: idk
        * Seismometer: idk, ore?
        * Micrometerorite: orbital hazards
- [ ] Mining && ISRU
    - [ ] surface outpots on tiles
    - [ ] Ice mining (first), electrolysis
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
- [ ] Tech tree
    - figure this out. I'm thinking you start off with a small fabricator that can only make small parts, and eventually you invest in builindg more advanced orbital/surface factories that can build complex stuff
- [ ] Misc polish
    - [ ] allow aero capture when payload has heat shield and user has atmospheric instrument readings
    - [ ] heat + cooling components in payloads
    - [ ] boiloff for cryo fuels, cooling systems which add weight and take power
    - [ ] different power sources
        * solar
        * nuclear
        * fuel cell
    - [ ] surface hazards: volcanoes, mountains, high winds, acidic atmospheres
    - [ ] orbital hazards: micrometeorites, high radiation, high magnetic fields
    - [ ] comm range limits, line-of-sight (to incentivize comm networks and relays)
    - [ ] re-usable first stages that return back to the bodys inventory after launch
    - [ ] craft should crash if they crash and don't have landing gear or parachutes+body atmos
    - [ ] craft should burn up if they enter atmosphere without heat shielding of some kind
    - [ ] tug payloads that transport fuel, raw materials or other craft
    - [ ] local biomes, informed by actual local conditions (rotation, tilt, lattitude)
    - [ ] planetary atmospheres, clouds, tile detail
    - [ ] asteroids
    - [ ] make actual building models, and rotate them to their tile's normal
    - [ ] show more info about stages, their dv, their resources, maybe a little model sprite, in VAB and factory
    - [ ] show how many stages we have in inventory in the factory