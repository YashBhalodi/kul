// A project for tests that need the tree to draw everything it can.

/**
 * A project that draws every category the tree has: all three genders, a
 * ghost (a past marriage with a child), birth, adoption, and both a current
 * and an ended marriage.
 */
export const EVERY_CATEGORY = `
person ana   name:"Ana"   gender:female
person ben   name:"Ben"   gender:male
person cal   name:"Cal"   gender:other
person dee   name:"Dee"   gender:female
marriage m_ana_ben ana ben  start:1970  end:1980  end_reason:divorce
person eve   name:"Eve"   gender:female
  birth m_ana_ben
marriage m_ana_cal ana cal  start:1982
person fay   name:"Fay"   gender:female
  birth m_ana_cal
person gus   name:"Gus"   gender:male
  adoption m_ana_cal start:1990
marriage m_ben_dee ben dee  start:1984
`
