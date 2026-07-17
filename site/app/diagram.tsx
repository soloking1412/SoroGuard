/**
 * The flow, drawn by hand. User registers a rule and arms a scoped policy. Keepers watch the
 * feeds and the ledger. On a trigger a keeper calls execute, the guardian re-derives the
 * trigger from the feed itself, and only then does the policy authorize the scoped exit.
 */
export default function Diagram() {
  return (
    <svg viewBox="0 0 760 430" role="img" aria-labelledby="diagram-title">
      <title id="diagram-title">
        SoroGuard architecture: the guardian re-derives a rule&apos;s trigger from the oracle
        before the user&apos;s policy authorizes a keeper to exit the position.
      </title>

      <defs>
        <marker
          id="arrow"
          viewBox="0 0 10 10"
          refX="9"
          refY="5"
          markerWidth="7"
          markerHeight="7"
          orient="auto-start-reverse"
        >
          <path d="M0,0 L10,5 L0,10 z" fill="#8a9198" />
        </marker>
        <marker
          id="arrow-accent"
          viewBox="0 0 10 10"
          refX="9"
          refY="5"
          markerWidth="7"
          markerHeight="7"
          orient="auto-start-reverse"
        >
          <path d="M0,0 L10,5 L0,10 z" fill="#4db6a5" />
        </marker>
      </defs>

      {/* Nodes */}
      <g className="node">
        <rect x="40" y="150" width="150" height="62" rx="7" />
        <text className="t" x="115" y="176">
          User
        </text>
        <text className="s" x="115" y="196">
          holds keys throughout
        </text>
      </g>

      <g className="node hub">
        <rect x="300" y="38" width="180" height="66" rx="7" />
        <text className="t" x="390" y="66">
          Guardian contract
        </text>
        <text className="s" x="390" y="86">
          re-derives the trigger
        </text>
      </g>

      <g className="node">
        <rect x="300" y="182" width="180" height="66" rx="7" />
        <text className="t" x="390" y="210">
          Guarded account
        </text>
        <text className="s" x="390" y="230">
          scoped policy
        </text>
      </g>

      <g className="node">
        <rect x="300" y="326" width="180" height="66" rx="7" />
        <text className="t" x="390" y="354">
          Position
        </text>
        <text className="s" x="390" y="374">
          Blend · Soroswap · Aquarius
        </text>
      </g>

      <g className="node">
        <rect x="560" y="38" width="160" height="56" rx="7" />
        <text className="t" x="640" y="62">
          Keeper network
        </text>
        <text className="s" x="640" y="81">
          permissionless
        </text>
      </g>

      <g className="node">
        <rect x="560" y="128" width="160" height="48" rx="7" />
        <text className="t" x="640" y="157">
          Soroban RPC
        </text>
      </g>

      <g className="node">
        <rect x="560" y="210" width="160" height="56" rx="7" />
        <text className="t" x="640" y="234">
          SEP-40 feeds
        </text>
        <text className="s" x="640" y="253">
          oracle prices
        </text>
      </g>

      {/* Edges */}
      {/* User registers a rule with the guardian */}
      <line className="edge" x1="190" y1="168" x2="300" y2="80" markerEnd="url(#arrow)" />
      <text className="l num" x="228" y="112">
        1 register
      </text>

      {/* User arms the scoped policy on their account */}
      <line className="edge" x1="190" y1="192" x2="300" y2="210" markerEnd="url(#arrow)" />
      <text className="l" x="205" y="222">
        arms policy
      </text>

      {/* Keeper calls execute on the guardian */}
      <line className="edge" x1="560" y1="70" x2="480" y2="70" markerEnd="url(#arrow)" />
      <text className="l num" x="497" y="60">
        2 execute
      </text>

      {/* Keeper watches RPC and feeds */}
      <line className="edge dashed" x1="628" y1="94" x2="628" y2="128" markerEnd="url(#arrow)" />
      <line className="edge dashed" x1="662" y1="94" x2="662" y2="210" markerEnd="url(#arrow)" />
      <text className="l" x="690" y="120">
        watches
      </text>

      {/* Guardian re-derives the trigger from the feed. The load-bearing arrow. */}
      <line
        className="edge accent"
        x1="480"
        y1="92"
        x2="560"
        y2="232"
        markerEnd="url(#arrow-accent)"
      />
      <text className="l accent" x="486" y="168">
        3 re-derive
      </text>

      {/* Guardian authorizes through the policy */}
      <line className="edge" x1="390" y1="104" x2="390" y2="182" markerEnd="url(#arrow)" />
      <text className="l num" x="398" y="148">
        4 authorize
      </text>

      {/* Policy releases the scoped exit into the position */}
      <line className="edge" x1="390" y1="248" x2="390" y2="326" markerEnd="url(#arrow)" />
      <text className="l" x="398" y="292">
        scoped exit
      </text>
    </svg>
  );
}
