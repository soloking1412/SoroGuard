/** The SoroGuard mark: a shield (the guard) with the floor line you set drawn through it. */
export default function Logo({ size = 26 }: { size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 32 32"
      fill="none"
      role="img"
      aria-label="SoroGuard"
    >
      <rect x="1" y="1" width="30" height="30" rx="8" fill="#4DB6A5" />
      <path
        d="M16 5.5 L23 8.4 V15.4 C23 20 20 23.3 16 25 C12 23.3 9 20 9 15.4 V8.4 Z"
        fill="#0B0D0E"
      />
      <rect x="10.6" y="16.4" width="10.8" height="2.4" rx="1.2" fill="#4DB6A5" />
    </svg>
  );
}
