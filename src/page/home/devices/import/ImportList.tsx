import { useI18n } from '@/common/i18n/useI18n.ts';
import type { DeviceImportItem } from '@/common/model/DeviceImport.ts';
import { itemKey, matchItem } from '@/page/common/util/deviceImportUtil.ts';
import { CheckBox, CheckBoxOutlineBlank, IndeterminateCheckBox, Search } from '@mui/icons-material';
import { Box, ButtonBase, InputBase, Table, TableBody, TableCell, TableHead, TableRow, Typography } from '@mui/material';

interface ImportListProps {
  query: string;
  selectedKeys: string[];
  items: DeviceImportItem[];
  onToggle: (key: string) => void;
  onQueryChange: (query: string) => void;
  onChangeSelectedKeys: (keys: string[]) => void;
}

export function ImportList(props: ImportListProps) {
  const { t, tf } = useI18n();
  const keyword = props.query.trim().toLowerCase();
  const filtered = keyword ? props.items.filter(item => matchItem(item, keyword)) : props.items;
  const selectableKeys = filtered.filter(item => !item.exists).map(itemKey);
  const selectedInView = selectableKeys.filter(key => props.selectedKeys.includes(key));
  const allSelected = selectableKeys.length > 0 && selectedInView.length === selectableKeys.length;
  const someSelected = selectedInView.length > 0 && !allSelected;
  const { items, query, selectedKeys, onToggle, onQueryChange, onChangeSelectedKeys } = props;

  const toggleAll = (): void => {
    if (allSelected) {
      const drop = new Set(selectableKeys);
      onChangeSelectedKeys(selectedKeys.filter(key => !drop.has(key)));
      return;
    }
    onChangeSelectedKeys([...new Set([...selectedKeys, ...selectableKeys])]);
  };

  const renderHeadCheck = () => {
    if (allSelected) {
      return <CheckBox className="tm-job-table-check" />;
    }
    if (someSelected) {
      return <IndeterminateCheckBox className="tm-job-table-check" />;
    }
    return <CheckBoxOutlineBlank className="tm-job-table-check" />;
  };

  return (
    <>
      <Box className="tm-home-searchbar tm-job-dialog-search">
        <Search className="tm-home-search-icon" />
        <InputBase className="tm-home-search-input-root" inputProps={{ className: 'tm-home-search-input' }} placeholder={t('home.devices.importSearch')} value={query} onChange={event => onQueryChange(event.target.value)} />
      </Box>
      <Typography className="tm-job-muted">{tf('home.devices.importSummary', { total: items.length, filtered: filtered.length, selected: selectedInView.length })}</Typography>
      <Box className="tm-job-table-wrap tm-home-import-list">
        {filtered.length === 0 ? (
          <Typography className="tm-job-muted" sx={{ p: 1.5 }}>
            {t('home.devices.importNoMatch')}
          </Typography>
        ) : (
          <Table stickyHeader size="small" className="tm-job-device-table tm-home-import-table">
            <TableHead>
              <TableRow>
                <TableCell className="tm-job-table-col-check">
                  <ButtonBase disableRipple className="tm-job-table-select-all" onClick={toggleAll} aria-label={allSelected ? t('home.common.deselectAll') : t('home.common.selectAll')}>
                    {renderHeadCheck()}
                  </ButtonBase>
                </TableCell>
                <TableCell className="tm-home-import-col-name">{t('home.common.name')}</TableCell>
                <TableCell className="tm-home-import-col-host">{t('home.common.host')}</TableCell>
                <TableCell className="tm-home-import-col-user">{t('home.devices.importUser')}</TableCell>
                <TableCell className="tm-home-import-col-jump">{t('home.devices.importJump')}</TableCell>
              </TableRow>
            </TableHead>
            <TableBody>
              {filtered.map(item => {
                const key = itemKey(item);
                const selected = selectedKeys.includes(key);
                return (
                  <TableRow key={key} hover selected={selected} className={item.exists ? 'tm-job-table-row is-exists' : selected ? 'tm-job-table-row is-selected' : 'tm-job-table-row'} onClick={() => (item.exists ? undefined : onToggle(key))}>
                    <TableCell className="tm-job-table-col-check">
                      <Box className="tm-job-table-check-slot">{selected ? <CheckBox className="tm-job-table-check" /> : <CheckBoxOutlineBlank className="tm-job-table-check" />}</Box>
                    </TableCell>
                    <TableCell className="tm-home-import-col-name">
                      <Box className="tm-home-import-name">
                        <Typography component="span" className="tm-home-import-name-text">
                          {item.name}
                        </Typography>
                        {item.exists ? (
                          <Typography component="span" className="tm-home-import-exists">
                            {t('home.devices.importExists')}
                          </Typography>
                        ) : null}
                      </Box>
                    </TableCell>
                    <TableCell className="tm-home-import-col-host">
                      {item.host}:{item.port}
                    </TableCell>
                    <TableCell className="tm-home-import-col-user">{item.username}</TableCell>
                    <TableCell className="tm-home-import-col-jump">{item.jump_name}</TableCell>
                  </TableRow>
                );
              })}
            </TableBody>
          </Table>
        )}
      </Box>
    </>
  );
}
