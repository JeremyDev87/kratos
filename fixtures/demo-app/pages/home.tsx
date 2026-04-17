import { sum } from "../src/lib/math";
import { LiveCard } from "../src/components/LiveCard";

export default function HomePage() {
  return <main>{sum(3, 4)} <LiveCard /></main>;
}
