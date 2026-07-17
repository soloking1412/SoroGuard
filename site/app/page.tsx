import Diagram from "./diagram";
import Logo from "./logo";
import ScrollReveal from "./scroll-reveal";

const REPO = "https://github.com/soloking1412/soroguard";
const GITHUB = "https://github.com/soloking1412";
const AUDIT = "https://audits.sherlock.xyz/watson/soloking";
const EMAIL = "maheswar141203@gmail.com";

// External links open in a new tab. In-page "#" anchors stay in this tab.
const ext = { target: "_blank", rel: "noopener noreferrer" } as const;

export default function Home() {
  return (
    <>
      <ScrollReveal />

      <nav className="nav">
        <div className="wrap nav-inner">
          <a className="brand" href="#top">
            <Logo />
            SoroGuard
          </a>
          <div className="nav-links">
            <a href="#how" data-optional>
              How it works
            </a>
            <a href="#architecture">Architecture</a>
            <a href="#status" data-optional>
              Status
            </a>
            <a href={REPO} {...ext}>
              GitHub
            </a>
          </div>
        </div>
      </nav>

      <main id="top">
        <header className="wrap hero">
          <h1>Stop-loss and auto-exit for Stellar DeFi.</h1>
          <p className="lede">
            You set the rule. If it triggers, keepers pull your funds out before you lose them.
            Non-custodial. You keep your keys.
          </p>
          <p className="subnote">
            Soroban has no built-in automation. SoroGuard adds the safety layer users have been
            missing.
          </p>
          <div className="cta">
            <a className="btn btn-primary" href="#architecture">
              Read the architecture
            </a>
            <a className="btn btn-ghost" href={REPO} {...ext}>
              View on GitHub
            </a>
          </div>
        </header>

        <section id="why">
          <div className="wrap reveal">
            <p className="eyebrow">Why this exists</p>
            <h2>A drain nobody could step out of.</h2>
            <p>
              On February 22, 2026, an oracle manipulation attack drained about $10.8M from the
              YieldBlox pool on Blend. A single trade in an illiquid market moved USTRY from
              about $1 to $106. The oracle read that trade as the price. The attacker borrowed
              against collateral the pool now badly overvalued, and the pool&apos;s depositors
              were the ones left short.
            </p>
            <p>
              The root cause was the pool&apos;s oracle configuration, not Blend&apos;s core
              contracts. That distinction matters, because it means any pool can be next.
            </p>
            <p>
              What followed was people. Stellar validators coordinated over a weekend and froze
              about $7.5M of it before it could be bridged out. The rest needed a bounty offer.
              No depositor had a way to say &quot;if this position gets dangerous, get me
              out.&quot;
            </p>
            <p>
              Every mature chain has that. Ethereum has stop-loss bots and automation networks
              like Gelato and Chainlink Automation. Stellar doesn&apos;t, because Soroban
              contracts can&apos;t trigger themselves. Someone outside has to act, and right now
              nobody does. SoroGuard is that outside actor, made permissionless and safe.
            </p>
          </div>
        </section>

        <section id="how">
          <div className="wrap reveal">
            <p className="eyebrow">How it works</p>
            <h2>Four moving parts. You control all of them.</h2>
            <div className="steps">
              <div className="step">
                <span className="step-n">01</span>
                <div>
                  <h3>Set a rule.</h3>
                  <p>
                    Pick a position and a condition. Sell if price drops under X. Exit if your
                    loan health falls below Y. Pull out if the oracle deviates past Z.
                  </p>
                </div>
              </div>
              <div className="step">
                <span className="step-n">02</span>
                <div>
                  <h3>Rule lives on-chain.</h3>
                  <p>
                    Your rule is stored in the guardian contract and enforced through a
                    smart-account policy scoped to only the actions it needs. No keys leave your
                    control.
                  </p>
                </div>
              </div>
              <div className="step">
                <span className="step-n">03</span>
                <div>
                  <h3>Keepers watch.</h3>
                  <p>
                    A permissionless network watches SEP-40 oracle prices and ledger events.
                    When your condition is met, a keeper submits the exit.
                  </p>
                </div>
              </div>
              <div className="step">
                <span className="step-n">04</span>
                <div>
                  <h3>Guardian verifies, then acts.</h3>
                  <p>
                    Before anything runs, the guardian re-checks the trigger against on-chain
                    data. If it&apos;s real, your funds are pulled out. If not, nothing happens.
                  </p>
                </div>
              </div>
            </div>
          </div>
        </section>

        <section id="architecture">
          <div className="wrap reveal">
            <p className="eyebrow">Architecture</p>
            <h2>The guardian never trusts the keeper.</h2>

            <div className="diagram">
              <Diagram />
            </div>

            <div className="parts">
              <div className="part">
                <h3>
                  Guardian contract<span className="tag">Soroban / Rust</span>
                </h3>
                <p>
                  Stores each user&apos;s rules. Before any protective action runs, it verifies
                  the keeper&apos;s claimed trigger against real on-chain data. Holds no funds.
                </p>
              </div>
              <div className="part">
                <h3>
                  Smart-account policy<span className="tag">Soroban smart accounts</span>
                </h3>
                <p>
                  Scopes exactly what the keeper may do on the user&apos;s account: only call
                  withdraw, repay or swap-out on the protocols the user is in, and only when a
                  rule condition actually holds. This is what keeps it non-custodial.
                </p>
              </div>
              <div className="part">
                <h3>
                  Keeper network<span className="tag">off-chain</span>
                </h3>
                <p>
                  Permissionless watchers. They read SEP-40 prices and Soroban RPC ledger
                  events, spot a triggered rule, and submit the protective transaction. Paid a
                  small fee on correct execution. Anyone can run one.
                </p>
              </div>
              <div className="part">
                <h3>
                  Rule engine<span className="tag">v1</span>
                </h3>
                <p>
                  Three rule types: a price stop-loss, a lending health-factor floor, and an
                  oracle-deviation exit. The deviation rule is the direct lesson of YieldBlox: a
                  cross-feed check would have flagged USTRY at $106.
                </p>
              </div>
            </div>

            <div className="code" aria-label="An example SoroGuard rule">
              <pre>
                <span className="c">// a stop-loss on an XLM position, priced through a SEP-40 feed</span>
                {"\n"}
                <span className="k">Rule</span> {"{"}
                {"\n"}    owner:    G...USER,
                {"\n"}    position: soroswap_lp,
                {"\n"}    trigger:  <span className="k">Price</span> {"{"} asset: XLM, below:{" "}
                <span className="s">0.11</span>, oracle: sep40_feed {"}"},
                {"\n"}    action:   <span className="k">ExitToStable</span> {"{"} to: USDC {"}"},
                {"\n"}    cooldown: <span className="s">300</span>,{"  "}
                <span className="c">// seconds, guards against a single-ledger wick</span>
                {"\n"}
                {"}"}
              </pre>
            </div>
            <p className="caption">
              The guardian re-derives the trigger from oracle and ledger state before it
              authorizes the action. It calls the public functions of Blend, Aquarius or
              Soroswap the same way any wallet does, using the user&apos;s own authorization. No
              partnerships required.
            </p>
          </div>
        </section>

        <section id="custody">
          <div className="wrap reveal">
            <p className="eyebrow">Non-custodial</p>
            <h2>It never holds your funds.</h2>
            <p>
              SoroGuard has no vault and no custody. Your position stays where it is. The only
              thing SoroGuard can do is execute the exact exit you defined, when the condition
              you set is actually true, using a smart-account policy you can revoke any time.
            </p>
            <p>
              <strong>If SoroGuard disappeared tomorrow, your funds would be untouched.</strong>
            </p>
          </div>
        </section>

        <section id="status">
          <div className="wrap reveal">
            <p className="eyebrow">Status</p>
            <h2>Built in the open.</h2>
            <div className="status-list">
              <div className="status-row">
                <span className="badge badge-done">Implemented</span>
                <span className="label">Guardian contract and policy model</span>
              </div>
              <div className="status-row">
                <span className="badge badge-done">Implemented</span>
                <span className="label">
                  All three rule types: stop-loss, health-factor, oracle-deviation
                </span>
              </div>
              <div className="status-row">
                <span className="badge badge-progress">In progress</span>
                <span className="label">Keeper client, submit path not yet run on live RPC</span>
              </div>
              <div className="status-row">
                <span className="badge badge-planned">Next</span>
                <span className="label">Testnet deployment</span>
              </div>
              <div className="status-row">
                <span className="badge badge-planned">Planned</span>
                <span className="label">Mainnet, after an audit</span>
              </div>
            </div>
            <p className="caption" style={{ marginTop: "1.25rem" }}>
              Follow the{" "}
              <a href={REPO} {...ext}>
                repo
              </a>{" "}
              for progress.
            </p>
          </div>
        </section>

        <section id="builder">
          <div className="wrap reveal">
            <p className="eyebrow">Who&apos;s building this</p>
            <h2>Maheswaran Velmurugan, developer and auditor.</h2>
            <p>
              I build blockchain developer tooling and DeFi protocols, and I audit them. On the
              security side I have 15+ confirmed High and Medium findings, including lending and
              vault protocols.
            </p>
            <p>
              SoroGuard is a rebuild of avaguard, a circuit-breaker and invariant-monitoring
              system I wrote for Avalanche, adapted to Soroban. I also built Stylus-Toolkit, a
              CLI to build, deploy and profile Rust-to-WASM contracts, the same contract model
              Soroban uses. It proved a 32.6% average gas saving and shipped under an Arbitrum
              Foundation grant. I&apos;ve also delivered a grant from the Stacks Foundation.
            </p>
            <div className="links-inline">
              <a href={AUDIT} {...ext}>
                Audit profile
              </a>
              <a href={GITHUB} {...ext}>
                GitHub
              </a>
            </div>
          </div>
        </section>
      </main>

      <footer>
        <div className="wrap">
          <div className="footer-links">
            <a href={REPO} {...ext}>
              GitHub
            </a>
            <a href={`${REPO}#readme`} {...ext}>
              Docs
            </a>
            <a href={`mailto:${EMAIL}`}>Contact</a>
          </div>
          <p className="fine">
            MIT licensed. Built on Stellar and Soroban.
            <br />
            No analytics. This site loads no third-party scripts and phones nothing home.
          </p>
        </div>
      </footer>
    </>
  );
}
