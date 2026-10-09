use windows_version::OsVersion;

pub(crate) fn windows8() ->bool
{
    OsVersion::current().major>8
}
