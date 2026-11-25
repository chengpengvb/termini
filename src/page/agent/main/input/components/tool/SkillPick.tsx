import { Search } from '@mui/icons-material';
import { useI18n } from '@/common/i18n/useI18n.ts';
import { useEffect, useRef, useState } from 'react';
import type { AiSkill } from '@/common/model/AiSkill.ts';
import { isEmpty, isNotEmpty } from '@/common/util/StringUtil.ts';
import { CHAT_TARGET_TYPE } from '@/common/const/ChatTargetConst.ts';
import { Box, ButtonBase, InputBase, Typography } from '@mui/material';
import type { ChatReference } from '@/common/model/dto/ChatReference.ts';

interface SkillPickProps {
  skills: AiSkill[];
  onPick: (ref: ChatReference) => void;
}

export function SkillPick(props: SkillPickProps) {
  const { t } = useI18n();
  const { skills, onPick } = props;
  const searchInputRef = useRef<HTMLInputElement | null>(null);
  const [keyword, setKeyword] = useState<string | null>(null);

  function resolveKeywordValue(): string {
    if (isNotEmpty(keyword)) {
      return keyword;
    }
    return '';
  }

  function resolveFilteredSkills(): AiSkill[] {
    if (isEmpty(keyword)) {
      return skills;
    }
    const value = keyword.toLowerCase();
    return skills.filter(skill => {
      if (isNotEmpty(skill.name) && skill.name.toLowerCase().includes(value)) {
        return true;
      }
      if (isNotEmpty(skill.description) && skill.description.toLowerCase().includes(value)) {
        return true;
      }
      if (isEmpty(skill.id)) {
        return false;
      }
      return skill.id.toLowerCase().includes(value);
    });
  }

  function handleKeywordChange(raw: string): void {
    if (isEmpty(raw)) {
      setKeyword(null);
      return;
    }
    setKeyword(raw);
  }

  function handlePick(skill: AiSkill): void {
    if (isEmpty(skill.id)) {
      return;
    }
    if (isEmpty(skill.name)) {
      return;
    }
    onPick({
      kind: CHAT_TARGET_TYPE.SKILL,
      id: skill.id,
      title: skill.name,
      data: null,
    });
  }

  useEffect(() => {
    searchInputRef.current?.focus({ preventScroll: true });
  }, []);

  const filtered = resolveFilteredSkills();

  return (
    <Box className="tm-agent-mount-picker">
      <Box className="tm-agent-mount-search-row">
        <Search className="tm-agent-mount-search-icon" />
        <InputBase inputRef={searchInputRef} value={resolveKeywordValue()} placeholder={t('agent.mount.searchSkill')} onChange={e => handleKeywordChange(e.target.value)} className="tm-agent-mount-search" />
      </Box>
      <Box className="tm-agent-mount-list">
        {skills.length === 0 ? (
          <Typography className="tm-agent-mount-empty">{t('agent.mount.noSkills')}</Typography>
        ) : filtered.length === 0 ? (
          <Typography className="tm-agent-mount-empty">{t('agent.mount.noMatch')}</Typography>
        ) : (
          filtered.map(skill => (
            <ButtonBase disableRipple key={skill.id} onClick={() => handlePick(skill)} className="tm-agent-mount-item">
              <Typography className="tm-agent-mount-item-name">{skill.name}</Typography>
            </ButtonBase>
          ))
        )}
      </Box>
    </Box>
  );
}
