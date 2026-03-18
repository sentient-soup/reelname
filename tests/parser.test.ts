import { describe, it, expect } from "vitest";
import { parseFileName } from "@/lib/parser";

describe("parseFileName", () => {
  describe("standard S01E01 patterns", () => {
    it("parses S01E01 format", () => {
      const result = parseFileName("Breaking.Bad.S05E16.1080p.BluRay.x264.mkv");
      expect(result.title).toBe("Breaking Bad");
      expect(result.season).toBe(5);
      expect(result.episode).toBe(16);
      expect(result.mediaType).toBe("tv");
      expect(result.quality).toBe("1080p");
    });

    it("parses lowercase s01e01", () => {
      const result = parseFileName("the.office.s02e03.720p.mkv");
      expect(result.season).toBe(2);
      expect(result.episode).toBe(3);
      expect(result.mediaType).toBe("tv");
    });

    it("parses 1x01 format", () => {
      const result = parseFileName("Seinfeld.3x12.The.Red.Dot.mkv");
      expect(result.season).toBe(3);
      expect(result.episode).toBe(12);
      expect(result.mediaType).toBe("tv");
    });

    it('parses "Season 1 Episode 1" format', () => {
      const result = parseFileName("Show Name Season 2 Episode 10.mkv");
      expect(result.season).toBe(2);
      expect(result.episode).toBe(10);
    });
  });

  describe("episode-only patterns (no season)", () => {
    it("parses E01 format and defaults to season 1", () => {
      const result = parseFileName("Show.Name.E05.Episode.Title.mkv");
      expect(result.season).toBe(1);
      expect(result.episode).toBe(5);
      expect(result.mediaType).toBe("tv");
    });

    it("parses Ep01 format and defaults to season 1", () => {
      const result = parseFileName("Show Name - Ep12 - Title.mkv");
      expect(result.season).toBe(1);
      expect(result.episode).toBe(12);
      expect(result.mediaType).toBe("tv");
    });
  });

  describe("bare episode number (anime-style)", () => {
    it("parses bare number between dashes", () => {
      const result = parseFileName("[Anime Time] Trigun - 001 - H.T. [Creditless Opening].mkv");
      expect(result.season).toBe(1);
      expect(result.episode).toBe(1);
      expect(result.mediaType).toBe("tv");
    });

    it("parses two-digit bare number between dashes", () => {
      const result = parseFileName("[SubGroup] Cowboy Bebop - 05 - Ballad of Fallen Angels.mkv");
      expect(result.season).toBe(1);
      expect(result.episode).toBe(5);
      expect(result.mediaType).toBe("tv");
    });

    it("parses bare number at end of name", () => {
      const result = parseFileName("[Fansub] Samurai Champloo - 26.mkv");
      expect(result.season).toBe(1);
      expect(result.episode).toBe(26);
      expect(result.mediaType).toBe("tv");
    });

    it("parses 4-digit episode numbers for long-running series", () => {
      const result = parseFileName("[HorribleSubs] One Piece - 1000 [1080p].mkv");
      expect(result.season).toBe(1);
      expect(result.episode).toBe(1000);
      expect(result.mediaType).toBe("tv");
    });

    it("extracts clean title from anime-style naming", () => {
      const result = parseFileName("[Group] Steins;Gate - 12 [1080p][HEVC].mkv");
      expect(result.season).toBe(1);
      expect(result.episode).toBe(12);
    });
  });

  describe("movies", () => {
    it("parses movie with year", () => {
      const result = parseFileName("Inception.2010.1080p.BluRay.x264.DTS.mkv");
      expect(result.title).toBe("Inception");
      expect(result.year).toBe(2010);
      expect(result.season).toBeUndefined();
      expect(result.episode).toBeUndefined();
      expect(result.mediaType).toBe("movie");
    });

    it("parses movie with year in parentheses", () => {
      const result = parseFileName("The Matrix (1999) 2160p BluRay.mkv");
      expect(result.year).toBe(1999);
      expect(result.mediaType).toBe("movie");
      expect(result.quality).toBe("2160p");
    });
  });

  describe("technical metadata extraction", () => {
    it("extracts codec", () => {
      const result = parseFileName("Show.S01E01.x265.mkv");
      expect(result.codec).toBe("x265");
    });

    it("extracts audio", () => {
      const result = parseFileName("Movie.2020.DTS-HD.MA.mkv");
      expect(result.audio).toMatch(/DTS/i);
    });

    it("extracts source", () => {
      const result = parseFileName("Show.S01E01.WEBRip.mkv");
      expect(result.source).toMatch(/WEB/i);
    });
  });

  describe("leading episode number", () => {
    it("parses leading number followed by dash and title", () => {
      const result = parseFileName("01 - Show Episode Title.mkv");
      expect(result.season).toBe(1);
      expect(result.episode).toBe(1);
      expect(result.mediaType).toBe("tv");
    });

    it("parses two-digit leading number", () => {
      const result = parseFileName("12 - The Big Episode.mkv");
      expect(result.season).toBe(1);
      expect(result.episode).toBe(12);
      expect(result.mediaType).toBe("tv");
    });

    it("parses three-digit leading number", () => {
      const result = parseFileName("100 - Hundredth Episode Title.mkv");
      expect(result.season).toBe(1);
      expect(result.episode).toBe(100);
      expect(result.mediaType).toBe("tv");
    });

    it("does not treat a leading year as an episode", () => {
      const result = parseFileName("2023 - Some Movie Title.mkv");
      expect(result.episode).toBeUndefined();
      expect(result.year).toBe(2023);
      expect(result.mediaType).toBe("movie");
    });

    it("extracts title from after the leading episode number", () => {
      const result = parseFileName("05 - My Great Episode.mkv");
      expect(result.episode).toBe(5);
      expect(result.title).toBe("My Great Episode");
    });
  });

  describe("edge cases", () => {
    it("S01E01 takes priority over bare number", () => {
      const result = parseFileName("Show - 03 - Title S02E05.mkv");
      expect(result.season).toBe(2);
      expect(result.episode).toBe(5);
    });

    it("does not treat a 4-digit year as an episode", () => {
      const result = parseFileName("Movie Name 2023.mkv");
      expect(result.year).toBe(2023);
      expect(result.episode).toBeUndefined();
      expect(result.mediaType).toBe("movie");
    });

    it("strips bracketed tags without polluting title", () => {
      const result = parseFileName("[SubGroup] My Show - 01 [720p][HEVC].mkv");
      expect(result.title).toBe("My Show");
      expect(result.episode).toBe(1);
    });
  });
});
