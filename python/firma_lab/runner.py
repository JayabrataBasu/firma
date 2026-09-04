"""firma_lab.runner -- expand designs into jobs, execute, collect (manual
Sec 23.1).

**Phase 3 stub.** Needs an ``ExperimentSpec`` (``firma_lab.spec``, also
stubbed) to expand into runs, and would drive ``firma-kernel`` through
``firma-registry`` to execute them -- the reason the manual's Sec 18.1
table lists ``firma-py <- ... firma-kernel, firma-registry``. Stage 7
deliberately does not add those two crates as dependencies of
``firma-py`` (ADR 0045's Note): an unused dependency now would be
designing for a hypothetical future requirement. Added when this module is
actually implemented.
"""
